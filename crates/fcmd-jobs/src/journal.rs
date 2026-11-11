//! A copy that survives a crash: the job journal (SPEC 03 §5a).
//!
//! A copy or a move writes down what it was asked to do — the sources, the destination, the
//! options — and, as it goes, which files it has finished. The file is removed when the job
//! ends, however it ends; one that is still there when the program starts again belongs to a
//! job that never got to its end, because the program or the machine died under it. That job
//! can then be taken up where it stopped: the files the journal names as done are passed over,
//! and the one it was in the middle of goes on from its `.fcmd-part` (§3c).
//!
//! Three things shaped the file:
//!
//! - **It is rewritten whole and renamed into place**, never appended to. A crash is the very
//!   thing this is for, and a file that can be half a line long at the moment of one is a file
//!   that has to be repaired before it can be believed.
//! - **It is written per file and not per byte**, and no more than a few times a second. A
//!   tree of small files would otherwise spend longer on its journal than on its copies; the
//!   price is that a file finished in the last instant before a crash may not be marked, and
//!   is then met again as a file that is already there.
//! - **It is text**, one fact per line, so that somebody who finds one can read what it was
//!   about — and so that this crate needs no serialiser it does not already have.
//!
//! Whether a journal's job is still running is answered by a lock on a file beside it rather
//! than by a process id: a second copy of the program started while the first is copying must
//! not offer to resume a job that is doing perfectly well, and a lock is the one thing the
//! system itself takes away when a process dies.

use std::collections::HashSet;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use fcmd_core::filter::Groups;
use fcmd_core::VPath;

use crate::fs::{part_path, JobFs};
use crate::spec::{ConflictPolicy, ErrorPolicy, JobKind, JobOptions, JobSpec};

/// The first line of every journal, which is how a file that is something else is told apart.
const MAGIC: &str = "fcmd-journal 1";
/// What a journal's file is called after its job's name.
pub const EXTENSION: &str = "journal";
/// The file beside it that its job keeps locked for as long as it runs.
const LOCK_EXTENSION: &str = "lock";
/// The least time between two writes: a few times a second, as the module says.
const EVERY: Duration = Duration::from_millis(250);

/// Why a file in the journals' folder was not taken for a journal.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum JournalError {
    #[error("{0}")]
    Io(String),
    #[error("not a job journal")]
    Foreign,
    #[error("line {line}: {what}")]
    Line { line: usize, what: String },
    #[error("it names no source or no destination")]
    Incomplete,
}

/// What a job wrote down about itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Journal {
    pub kind: JobKind,
    pub sources: Vec<VPath>,
    pub target: VPath,
    pub options: JobOptions,
    /// When the job was started, in seconds since 1970, for the list that offers it.
    pub started: u64,
    /// The destinations of the files that are finished.
    pub done: HashSet<VPath>,
    /// The destinations of the files that were begun and are not known to be finished: the
    /// one being written when the journal was last saved, and any that failed before it.
    /// Each may have left a `.fcmd-part` beside where it was going.
    pub open: Vec<VPath>,
}

impl Journal {
    /// The journal a job starts with, or `None` for a job that keeps none: anything but a
    /// copy or a move, a dry run, and a job with a path that would not read back as itself.
    ///
    /// The last is a name that is not text — which a local path may be, and which a line of a
    /// text file cannot carry. Such a job simply runs without a journal, as every job did
    /// before there was one.
    pub fn of(spec: &JobSpec, started: u64) -> Option<Journal> {
        if !matches!(spec.kind, JobKind::Copy | JobKind::Move) || spec.options.dry_run {
            return None;
        }
        let target = spec.target.clone()?;
        let round_trips = |p: &VPath| VPath::parse(&p.to_string()) == *p;
        if !spec.sources.iter().all(round_trips) || !round_trips(&target) {
            return None;
        }
        if let Some(root) = &spec.options.relative_to {
            if !round_trips(root) {
                return None;
            }
        }
        Some(Journal {
            kind: spec.kind,
            sources: spec.sources.clone(),
            target,
            options: spec.options.clone(),
            started,
            done: HashSet::new(),
            open: Vec::new(),
        })
    }

    /// The job to queue to take this one up again: the same sources, destination and options,
    /// with `resume` on whatever it was, because going on from the part is the point.
    pub fn spec(&self) -> JobSpec {
        let mut options = self.options.clone();
        options.resume = true;
        JobSpec::new(self.kind, self.sources.clone(), Some(self.target.clone()))
            .with_options(options)
    }

    /// The `.fcmd-part` files this journal knows may be lying about.
    pub fn parts(&self) -> Vec<VPath> {
        self.open.iter().map(part_path).collect()
    }

    pub fn encode(&self) -> String {
        let o = &self.options;
        let mut out = format!("{MAGIC}\n");
        let mut line = |key: &str, value: &str| {
            out.push_str(key);
            out.push(' ');
            out.push_str(&escape(value));
            out.push('\n');
        };
        let flag = |on: bool| if on { "yes" } else { "no" };
        line("kind", self.kind.as_str());
        line("started", &self.started.to_string());
        line("target", &self.target.to_string());
        for source in &self.sources {
            line("source", &source.to_string());
        }
        line("on-conflict", conflict_name(o.on_conflict));
        line("on-error", error_name(o.on_error));
        line("keep-mtime", flag(o.preserve.mtime));
        line("keep-mode", flag(o.preserve.mode));
        line("keep-xattrs", flag(o.preserve.xattrs));
        line("follow-symlinks", flag(o.follow_symlinks));
        line("only-newer", flag(o.only_newer));
        line("buffer", &o.buffer.to_string());
        line("system-copy", flag(o.system_copy));
        line("contents", flag(o.contents));
        line("verify", flag(o.verify));
        line("resume", flag(o.resume));
        if let Some(rate) = o.max_rate {
            line("max-rate", &rate.to_string());
        }
        if let Some(filter) = &o.filter {
            line("filter-case", flag(filter.case_sensitive()));
            line("filter", filter.source());
        }
        if let Some(root) = &o.relative_to {
            line("relative-to", &root.to_string());
        }
        for path in &self.open {
            line("open", &path.to_string());
        }
        // In a fixed order, so that two journals of the same state are the same bytes.
        let mut done: Vec<String> = self.done.iter().map(|p| p.to_string()).collect();
        done.sort();
        for path in &done {
            line("done", path);
        }
        // The last line says the file is whole: one cut short by a full disk or by a copy of
        // it gone wrong has no such line, and is refused rather than half believed.
        out.push_str("end\n");
        out
    }

    /// `groups` are the named mask lists a filter may use (`<arc>` and the user's own): the
    /// journal keeps the filter as it was typed, and it means what it meant only when read
    /// with the same names.
    pub fn parse(text: &str, groups: &Groups) -> Result<Journal, JournalError> {
        let mut lines = text.lines().enumerate();
        match lines.next() {
            Some((_, first)) if first == MAGIC => {}
            _ => return Err(JournalError::Foreign),
        }
        let mut kind = None;
        let mut target = None;
        let mut sources = Vec::new();
        let mut options = JobOptions::default();
        let mut started = 0;
        let mut done = HashSet::new();
        let mut open = Vec::new();
        let mut filter: Option<String> = None;
        let mut filter_case = false;
        let mut ended = false;
        for (at, raw) in lines {
            let bad = |what: &str| JournalError::Line {
                line: at + 1,
                what: what.to_string(),
            };
            if ended {
                return Err(bad("something after the end"));
            }
            if raw == "end" {
                ended = true;
                continue;
            }
            let (key, value) = raw.split_once(' ').unwrap_or((raw, ""));
            let value = unescape(value).ok_or_else(|| bad("a broken escape"))?;
            let flag = |v: &str| match v {
                "yes" => Ok(true),
                "no" => Ok(false),
                _ => Err(bad("neither yes nor no")),
            };
            match key {
                "kind" => {
                    kind = Some(match value.as_str() {
                        "copy" => JobKind::Copy,
                        "move" => JobKind::Move,
                        _ => return Err(bad("a kind of job that keeps no journal")),
                    })
                }
                "started" => started = value.parse().map_err(|_| bad("not a time"))?,
                "target" => target = Some(VPath::parse(&value)),
                "source" => sources.push(VPath::parse(&value)),
                "on-conflict" => {
                    options.on_conflict =
                        conflict_named(&value).ok_or_else(|| bad("an unknown conflict policy"))?
                }
                "on-error" => {
                    options.on_error =
                        error_named(&value).ok_or_else(|| bad("an unknown error policy"))?
                }
                "keep-mtime" => options.preserve.mtime = flag(&value)?,
                "keep-mode" => options.preserve.mode = flag(&value)?,
                "keep-xattrs" => options.preserve.xattrs = flag(&value)?,
                "follow-symlinks" => options.follow_symlinks = flag(&value)?,
                "only-newer" => options.only_newer = flag(&value)?,
                "buffer" => {
                    options.buffer = value
                        .parse::<usize>()
                        .ok()
                        .filter(|n| *n > 0)
                        .ok_or_else(|| bad("not a buffer size"))?
                }
                "system-copy" => options.system_copy = flag(&value)?,
                "contents" => options.contents = flag(&value)?,
                "verify" => options.verify = flag(&value)?,
                "resume" => options.resume = flag(&value)?,
                "max-rate" => {
                    options.max_rate = Some(value.parse().map_err(|_| bad("not a rate"))?)
                }
                "filter-case" => filter_case = flag(&value)?,
                "filter" => filter = Some(value),
                "relative-to" => options.relative_to = Some(VPath::parse(&value)),
                "open" => open.push(VPath::parse(&value)),
                "done" => {
                    done.insert(VPath::parse(&value));
                }
                // A line this version does not know is a journal from a later one, whose
                // meaning cannot be guessed at: a copy resumed on half its options is worse
                // than one not resumed.
                _ => return Err(bad("a line this version does not know")),
            }
        }
        if !ended {
            return Err(JournalError::Line {
                line: text.lines().count(),
                what: "the file stops before its end".to_string(),
            });
        }
        if let Some(source) = filter {
            let set =
                fcmd_core::MaskSet::parse_with(&source, groups, filter_case).map_err(|e| {
                    JournalError::Line {
                        line: 0,
                        what: e.to_string(),
                    }
                })?;
            options.filter = Some(set);
        }
        let (Some(kind), Some(target)) = (kind, target) else {
            return Err(JournalError::Incomplete);
        };
        if sources.is_empty() {
            return Err(JournalError::Incomplete);
        }
        Ok(Journal {
            kind,
            sources,
            target,
            options,
            started,
            done,
            open,
        })
    }

    pub fn read(path: &Path, groups: &Groups) -> Result<Journal, JournalError> {
        let bytes = std::fs::read(path).map_err(|e| JournalError::Io(e.to_string()))?;
        let text = String::from_utf8(bytes).map_err(|_| JournalError::Foreign)?;
        Journal::parse(&text, groups)
    }

    /// Write it beside where it goes and rename it into place, so that what is at `path` is
    /// always a whole journal: the old one or the new one, never part of either.
    pub fn write(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("tmp");
        let mut file = File::create(&tmp)?;
        file.write_all(self.encode().as_bytes())?;
        // Not synced to the disk: that would cost more than the copies of a tree of small
        // files, and what is lost without it is a few marks after a power cut — files the
        // resumed job then meets as already there, which is the case it handles anyway.
        drop(file);
        std::fs::rename(&tmp, path)
    }
}

fn conflict_name(policy: ConflictPolicy) -> &'static str {
    match policy {
        ConflictPolicy::Ask => "ask",
        ConflictPolicy::Overwrite => "overwrite",
        ConflictPolicy::OverwriteIfNewer => "newer",
        ConflictPolicy::OverwriteIfSizeDiffers => "size",
        ConflictPolicy::Skip => "skip",
        ConflictPolicy::Rename => "rename",
        ConflictPolicy::Append => "append",
    }
}

fn conflict_named(name: &str) -> Option<ConflictPolicy> {
    Some(match name {
        "ask" => ConflictPolicy::Ask,
        "overwrite" => ConflictPolicy::Overwrite,
        "newer" => ConflictPolicy::OverwriteIfNewer,
        "size" => ConflictPolicy::OverwriteIfSizeDiffers,
        "skip" => ConflictPolicy::Skip,
        "rename" => ConflictPolicy::Rename,
        "append" => ConflictPolicy::Append,
        _ => return None,
    })
}

fn error_name(policy: ErrorPolicy) -> &'static str {
    match policy {
        ErrorPolicy::Ask => "ask",
        ErrorPolicy::Skip => "skip",
        ErrorPolicy::Stop => "stop",
    }
}

fn error_named(name: &str) -> Option<ErrorPolicy> {
    Some(match name {
        "ask" => ErrorPolicy::Ask,
        "skip" => ErrorPolicy::Skip,
        "stop" => ErrorPolicy::Stop,
        _ => return None,
    })
}

/// A file name may hold a line break, and a line of this file may not.
fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    out
}

fn unescape(value: &str) -> Option<String> {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        out.push(match chars.next()? {
            '\\' => '\\',
            'n' => '\n',
            'r' => '\r',
            _ => return None,
        });
    }
    Some(out)
}

/// The job a journal describes, less what an earlier run already took away.
///
/// A move deletes each source as its copy lands, and a source moved whole by a rename is gone
/// in one step: planning the job again as it was first asked would stop at the first source
/// that is not there. For a move, a missing source is a finished one. A copy never removes a
/// source, so one missing there is a real error and is left for the plan to report.
pub fn remaining(fs: &dyn JobFs, mut spec: JobSpec) -> JobSpec {
    if spec.kind == JobKind::Move {
        spec.sources.retain(|source| fs.exists(source));
    }
    spec
}

/// Where a job is to keep its journal, and what it takes up again if it is a resumed one.
///
/// This is what travels from whoever queues the job to the thread that runs it: it is plain
/// data, and no file is touched until [`Recorder::begin`] is called on that thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ticket {
    /// The file this run writes.
    pub path: PathBuf,
    /// The journal a dead run left, with where it is: its marks are carried over, and its
    /// file is removed once this run's own is in place.
    pub resumes: Option<(PathBuf, Journal)>,
}

impl Ticket {
    /// A new job's: `<folder>/<owner>-<id>.journal`. The owner is what keeps two copies of
    /// the program, each counting its jobs from one, from writing over each other's.
    pub fn new(folder: &Path, owner: u32, id: u64) -> Ticket {
        Ticket {
            path: folder.join(format!("{owner}-{id}.{EXTENSION}")),
            resumes: None,
        }
    }

    pub fn resuming(mut self, from: PathBuf, journal: Journal) -> Ticket {
        self.resumes = Some((from, journal));
        self
    }
}

/// The journal of a job that is running: what [`crate::run::execute_recorded`] tells about
/// each file, and what writes the file down when enough time has passed.
#[derive(Debug)]
pub struct Recorder {
    path: PathBuf,
    journal: Journal,
    /// Held locked for as long as this lives, which is what says the job is.
    lock: Option<File>,
    every: Duration,
    last: Instant,
    /// How long the last write took. A journal of a hundred thousand files is megabytes, and
    /// writing that four times a second would be the job's main work: the next write waits
    /// twenty times as long as the last one took, so the journal never costs more than a
    /// twentieth of the time.
    cost: Duration,
    dirty: bool,
}

impl Recorder {
    /// Start keeping a journal for this job, or `None` when it keeps none ([`Journal::of`])
    /// or when the folder cannot be written — a job never fails for want of its journal.
    pub fn begin(ticket: Ticket, spec: &JobSpec) -> Option<Recorder> {
        let started = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let mut journal = Journal::of(spec, started)?;
        if let Some((_, old)) = &ticket.resumes {
            journal.done = old.done.clone();
            journal.open = old.open.clone();
        }
        std::fs::create_dir_all(ticket.path.parent()?).ok()?;
        let lock = File::create(ticket.path.with_extension(LOCK_EXTENSION)).ok();
        if let Some(lock) = &lock {
            // A filesystem that has no locks leaves the journal unguarded, which costs only
            // this: a second copy of the program may offer a job that is still running.
            let _ = lock.try_lock();
        }
        let mut recorder = Recorder {
            path: ticket.path,
            journal,
            lock,
            every: EVERY,
            last: Instant::now(),
            cost: Duration::ZERO,
            dirty: false,
        };
        // At once, before anything is planned: counting a large tree takes its time, and a
        // crash during it is still a job that was asked for and did not happen.
        recorder.save();
        if !recorder.path.exists() {
            recorder.finish();
            return None;
        }
        if let Some((old, _)) = &ticket.resumes {
            if *old != recorder.path {
                let _ = std::fs::remove_file(old);
                let _ = std::fs::remove_file(old.with_extension(LOCK_EXTENSION));
            }
        }
        Some(recorder)
    }

    /// Write after every change instead of a few times a second: for a test, which cannot
    /// wait a quarter of a second between two files.
    pub fn eagerly(mut self) -> Recorder {
        self.every = Duration::ZERO;
        self
    }

    pub fn journal(&self) -> &Journal {
        &self.journal
    }

    /// Did an earlier run of this job finish this file?
    pub fn is_done(&self, dst: &VPath) -> bool {
        self.journal.done.contains(dst)
    }

    /// A file is about to be written.
    pub fn opened(&mut self, dst: &VPath) {
        if !self.journal.open.contains(dst) {
            self.journal.open.push(dst.clone());
            self.dirty = true;
        }
        self.tick();
    }

    /// It is whole and in place.
    pub fn closed(&mut self, dst: &VPath) {
        self.journal.open.retain(|p| p != dst);
        self.journal.done.insert(dst.clone());
        self.dirty = true;
        self.tick();
    }

    /// It was passed over, or stopped by the user: nothing of it is left behind, so there is
    /// nothing to remember about it.
    pub fn dropped(&mut self, dst: &VPath) {
        self.journal.open.retain(|p| p != dst);
        self.dirty = true;
        self.tick();
    }

    /// Write the journal down if it has changed and it is time. Called from inside a file's
    /// copy as well, so that the one large file a job is in the middle of is on record as
    /// begun long before it ends.
    pub fn tick(&mut self) {
        if !self.dirty {
            return;
        }
        let wait = if self.every.is_zero() {
            Duration::ZERO
        } else {
            self.every.max(self.cost * 20)
        };
        if self.last.elapsed() >= wait {
            self.save();
        }
    }

    fn save(&mut self) {
        let before = Instant::now();
        // A write that fails is tried again at the next change: the job goes on either way.
        if self.journal.write(&self.path).is_ok() {
            self.dirty = false;
        }
        self.cost = before.elapsed();
        self.last = Instant::now();
    }

    /// The job has ended — well, badly or by `Esc`: there is nothing left to take up, so the
    /// journal goes, and its folder with it when that leaves it empty.
    pub fn finish(mut self) {
        let _ = std::fs::remove_file(&self.path);
        let _ = std::fs::remove_file(self.path.with_extension("tmp"));
        // Closed before it is removed: Windows will not delete a file somebody has open.
        self.lock = None;
        let _ = std::fs::remove_file(self.path.with_extension(LOCK_EXTENSION));
        if let Some(dir) = self.path.parent() {
            // Fails when anything else is in it, which is exactly when it should stay.
            let _ = std::fs::remove_dir(dir);
        }
    }

    /// Stop without tidying up, as a crash would: the journal stays as it was last written.
    /// For the tests of what a crash leaves; nothing in the program calls it.
    pub fn abandon(mut self) {
        self.lock = None;
    }
}

/// What the journals' folder holds at startup.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Left {
    /// The jobs that never reached their end, oldest first.
    pub journals: Vec<(PathBuf, Journal)>,
    /// The files that were not journals, each with why: to be said once, never to stop on.
    pub problems: Vec<(PathBuf, JournalError)>,
}

/// Read every journal a dead run left in `folder`.
///
/// A journal whose job is still running — its lock is held, by another copy of the program
/// or by this one — is not one of them. A folder that is not there holds nothing.
pub fn left_in(folder: &Path, groups: &Groups) -> Left {
    let mut left = Left::default();
    let Ok(read) = std::fs::read_dir(folder) else {
        return left;
    };
    let mut paths: Vec<PathBuf> = read
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == EXTENSION))
        .collect();
    paths.sort();
    for path in paths {
        if is_running(&path) {
            continue;
        }
        match Journal::read(&path, groups) {
            Ok(journal) => left.journals.push((path, journal)),
            Err(error) => left.problems.push((path, error)),
        }
    }
    left.journals.sort_by_key(|(_, j)| j.started);
    left
}

/// Is the job that writes this journal alive? It is when the lock beside it cannot be taken.
fn is_running(journal: &Path) -> bool {
    let Ok(lock) = File::open(journal.with_extension(LOCK_EXTENSION)) else {
        return false;
    };
    match lock.try_lock() {
        // Ours for the moment, so nobody else had it; dropping the handle gives it back.
        Ok(()) => false,
        Err(std::fs::TryLockError::WouldBlock) => true,
        // No locks on this filesystem: nothing can be said, and offering a job that may be
        // running is better than hiding one that is not.
        Err(std::fs::TryLockError::Error(_)) => false,
    }
}

/// Give a left-over job up: its journal goes, and so do the `.fcmd-part` files it names.
/// What was copied whole stays where it is — it is a file like any other by now.
pub fn discard(fs: &dyn JobFs, path: &Path, journal: &Journal) {
    for part in journal.parts() {
        // A part is only ever left on a disk of this machine (§3c): a server or an archive
        // writes its own way, and a path there is not one to go deleting by guesswork.
        if part.is_local() && !part.in_container() && fs.exists(&part) {
            let _ = fs.remove_file(&part);
        }
    }
    forget(path);
}

/// Remove a journal's own files and nothing else: for one that could not be read, which
/// names no part to take away.
pub fn forget(path: &Path) {
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(path.with_extension(LOCK_EXTENSION));
    let _ = std::fs::remove_file(path.with_extension("tmp"));
    if let Some(dir) = path.parent() {
        let _ = std::fs::remove_dir(dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn groups() -> Groups {
        fcmd_core::filter::default_groups()
    }

    use crate::spec::{Decision, Preserve, Question};
    use crate::{execute_recorded, plan, LocalFs, Progress, Reporter};
    use fcmd_core::CancelToken;

    /// Stops the job once so many files are through, the way a crash would: at a moment of
    /// its own choosing, between two files. And fails the test if it is ever asked anything.
    struct StopsAfter {
        files: u64,
        cancel: CancelToken,
    }

    impl Reporter for StopsAfter {
        fn progress(&mut self, progress: &Progress) {
            if progress.files_done >= self.files {
                self.cancel.cancel();
            }
        }
        fn ask(&mut self, question: Question) -> Decision {
            panic!("the job asked {question:?}");
        }
    }

    /// Every file under `root`, by its path from there, with its bytes.
    fn tree(root: &Path) -> Vec<(String, Vec<u8>)> {
        let mut out = Vec::new();
        let mut todo = vec![root.to_path_buf()];
        while let Some(dir) = todo.pop() {
            for entry in std::fs::read_dir(&dir).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    todo.push(path);
                } else {
                    let rel = path.strip_prefix(root).unwrap();
                    let rel: Vec<String> = rel
                        .components()
                        .map(|c| c.as_os_str().to_string_lossy().into_owned())
                        .collect();
                    out.push((rel.join("/"), std::fs::read(&path).unwrap()));
                }
            }
        }
        out.sort();
        out
    }

    /// A source tree of six files, one of them large enough to be worth a part.
    fn sources(root: &Path) {
        let src = root.join("src");
        std::fs::create_dir_all(src.join("deep/er")).unwrap();
        for (name, bytes) in [
            ("a.txt", &b"alpha"[..]),
            ("b.txt", b"beta"),
            ("c.txt", b"gamma"),
            ("deep/d.txt", b"delta"),
            ("deep/er/e.txt", b"epsilon"),
        ] {
            std::fs::write(src.join(name), bytes).unwrap();
        }
        let big: Vec<u8> = (0..300_000u32).map(|n| (n % 251) as u8).collect();
        std::fs::write(src.join("deep/er/big.bin"), big).unwrap();
    }

    /// Run `spec` with a journal and stop it after `files`, leaving the journal as a crash
    /// would. Says where the journal's folder is.
    fn run_and_die(root: &Path, spec: &JobSpec, files: u64) -> PathBuf {
        let folder = root.join("jobs");
        let mut recorder = Recorder::begin(Ticket::new(&folder, 1, 1), spec)
            .expect("it starts")
            .eagerly();
        let cancel = CancelToken::new();
        let mut reporter = StopsAfter {
            files,
            cancel: cancel.clone(),
        };
        let planned = plan(&LocalFs, spec, &cancel).unwrap();
        let outcome = execute_recorded(
            &LocalFs,
            spec,
            planned,
            &mut reporter,
            &cancel,
            Some(&mut recorder),
        );
        assert!(outcome.cancelled, "the job ran to its end: {outcome:?}");
        recorder.abandon();
        folder
    }

    /// Take up the one journal in `folder` and run it to its end.
    fn resume(folder: &Path) -> crate::Outcome {
        let mut left = left_in(folder, &groups());
        assert!(left.problems.is_empty(), "{left:?}");
        let (path, journal) = left.journals.pop().expect("a journal was left");
        let spec = remaining(&LocalFs, journal.spec());
        let ticket = Ticket::new(folder, 2, 1).resuming(path.clone(), journal);
        let mut recorder = Recorder::begin(ticket, &spec).expect("it starts again");
        assert!(
            !path.exists(),
            "the dead run's journal is replaced by this one's"
        );
        let cancel = CancelToken::new();
        let mut reporter = StopsAfter {
            files: u64::MAX,
            cancel: cancel.clone(),
        };
        let planned = plan(&LocalFs, &spec, &cancel).unwrap();
        let outcome = execute_recorded(
            &LocalFs,
            &spec,
            planned,
            &mut reporter,
            &cancel,
            Some(&mut recorder),
        );
        recorder.finish();
        outcome
    }

    #[test]
    fn a_copy_taken_up_from_its_journal_ends_as_an_uninterrupted_one_does() {
        let root = tempfile::tempdir().unwrap();
        sources(root.path());
        std::fs::create_dir_all(root.path().join("dst")).unwrap();
        let spec = spec(root.path());
        let folder = run_and_die(root.path(), &spec, 3);

        let copied = tree(&root.path().join("dst"));
        assert!(
            !copied.is_empty() && copied.len() < 6,
            "the job was meant to die halfway: {} files",
            copied.len()
        );
        // And the file it would have been writing, cut off as a kill leaves it: the first
        // part of its bytes under the part's name, if it had not got to it yet.
        let big = root.path().join("dst/src/deep/er/big.bin");
        if !big.exists() {
            let bytes = std::fs::read(root.path().join("src/deep/er/big.bin")).unwrap();
            std::fs::create_dir_all(big.parent().unwrap()).unwrap();
            std::fs::write(
                root.path().join("dst/src/deep/er/big.bin.fcmd-part"),
                &bytes[..100_000],
            )
            .unwrap();
        }

        let outcome = resume(&folder);
        assert!(outcome.is_clean(), "{outcome:?}");
        assert_eq!(
            outcome.files as usize,
            6 - copied.len(),
            "only what was left to do was done"
        );

        // The same copy, never interrupted, somewhere else.
        std::fs::create_dir_all(root.path().join("whole")).unwrap();
        let mut straight = spec.clone();
        straight.target = Some(VPath::local(root.path().join("whole")));
        let cancel = CancelToken::new();
        let planned = plan(&LocalFs, &straight, &cancel).unwrap();
        let mut quiet = StopsAfter {
            files: u64::MAX,
            cancel: cancel.clone(),
        };
        crate::execute(&LocalFs, &straight, planned, &mut quiet, &cancel);
        assert_eq!(
            tree(&root.path().join("dst")),
            tree(&root.path().join("whole"))
        );
        assert!(!folder.exists(), "a job that ended leaves no journal");
    }

    #[test]
    fn a_file_the_journal_calls_done_is_not_touched_again() {
        let root = tempfile::tempdir().unwrap();
        sources(root.path());
        std::fs::create_dir_all(root.path().join("dst")).unwrap();
        let folder = run_and_die(root.path(), &spec(root.path()), 2);
        // Somebody edited one of the copies between the crash and the restart. The policy
        // is to ask about a file that is already there, and the reporter fails the test if
        // asked: a finished file is neither asked about nor written over.
        let (name, _) = tree(&root.path().join("dst")).remove(0);
        let edited = root.path().join("dst").join(&name);
        std::fs::write(&edited, b"mine now").unwrap();

        let outcome = resume(&folder);
        assert!(outcome.is_clean(), "{outcome:?}");
        assert_eq!(std::fs::read(&edited).unwrap(), b"mine now");
    }

    #[test]
    fn a_move_taken_up_again_does_not_look_for_what_it_already_moved() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("dst")).unwrap();
        std::fs::write(root.path().join("still-here.txt"), b"to do").unwrap();
        let moving = JobSpec::new(
            JobKind::Move,
            vec![
                VPath::local(root.path().join("gone.txt")),
                VPath::local(root.path().join("still-here.txt")),
            ],
            Some(VPath::local(root.path().join("dst"))),
        );
        let left = remaining(&LocalFs, moving.clone());
        assert_eq!(left.sources, vec![moving.sources[1].clone()]);
        // A copy keeps the missing one, for the plan to complain about.
        let mut copying = moving.clone();
        copying.kind = JobKind::Copy;
        assert_eq!(remaining(&LocalFs, copying).sources.len(), 2);
    }

    fn spec(root: &Path) -> JobSpec {
        JobSpec::new(
            JobKind::Copy,
            vec![VPath::local(root.join("src"))],
            Some(VPath::local(root.join("dst"))),
        )
    }

    #[test]
    fn a_journal_reads_back_as_what_was_written() {
        let root = tempfile::tempdir().unwrap();
        let mut spec = spec(root.path());
        spec.kind = JobKind::Move;
        spec.options.on_conflict = ConflictPolicy::OverwriteIfNewer;
        spec.options.on_error = ErrorPolicy::Skip;
        spec.options.preserve = Preserve {
            mtime: false,
            mode: true,
            xattrs: false,
        };
        spec.options.verify = true;
        spec.options.contents = true;
        spec.options.max_rate = Some(4096);
        spec.options.filter = Some(fcmd_core::MaskSet::parse("*.rs,*.toml").unwrap());
        spec.options.relative_to = Some(VPath::local(root.path()));
        let mut journal = Journal::of(&spec, 1_700_000_000).expect("a move keeps a journal");
        // A name with a line break and a backslash in it, which is what the escaping is for.
        let odd = VPath::local(root.path().join("dst")).join("two\nlines\\here.txt");
        journal.done.insert(odd);
        journal
            .open
            .push(VPath::local(root.path().join("dst")).join("half.bin"));

        let read = Journal::parse(&journal.encode(), &groups()).expect("it reads back");
        assert_eq!(read, journal);
        assert_eq!(read.spec().options, spec.options);
    }

    #[test]
    fn only_a_copy_or_a_move_that_writes_keeps_a_journal() {
        let root = tempfile::tempdir().unwrap();
        let copy = spec(root.path());
        assert!(Journal::of(&copy, 0).is_some());
        let mut dry = copy.clone();
        dry.options.dry_run = true;
        assert!(Journal::of(&dry, 0).is_none(), "a dry run writes nothing");
        let delete = JobSpec::new(JobKind::Delete, copy.sources.clone(), None);
        assert!(Journal::of(&delete, 0).is_none());
    }

    #[test]
    fn a_file_that_is_not_a_journal_is_refused_with_a_reason() {
        assert_eq!(
            Journal::parse("[general]\n", &groups()),
            Err(JournalError::Foreign),
            "another program's file"
        );
        assert_eq!(Journal::parse("", &groups()), Err(JournalError::Foreign));
        // Cut short: the header is right and the last line is not there.
        let root = tempfile::tempdir().unwrap();
        let whole = Journal::of(&spec(root.path()), 0).unwrap().encode();
        let cut = &whole[..whole.len() - "end\n".len()];
        assert!(matches!(
            Journal::parse(cut, &groups()),
            Err(JournalError::Line { .. })
        ));
        // A line from a version that knows more than this one.
        let later = whole.replace("end\n", "encrypt yes\nend\n");
        assert!(matches!(
            Journal::parse(&later, &groups()),
            Err(JournalError::Line { .. })
        ));
        // Right in form and empty of anything to do.
        assert_eq!(
            Journal::parse(&format!("{MAGIC}\nkind copy\nend\n"), &groups()),
            Err(JournalError::Incomplete)
        );
    }

    #[test]
    fn a_finished_job_leaves_no_journal_and_no_folder() {
        let root = tempfile::tempdir().unwrap();
        let folder = root.path().join("jobs");
        let ticket = Ticket::new(&folder, 1, 7);
        let path = ticket.path.clone();
        let recorder = Recorder::begin(ticket, &spec(root.path())).expect("it starts");
        assert!(path.exists(), "the journal is written before the job plans");
        recorder.finish();
        assert!(!path.exists());
        assert!(!folder.exists(), "an empty folder is not left behind");
    }

    #[test]
    fn a_running_job_is_not_offered_and_a_dead_one_is() {
        let root = tempfile::tempdir().unwrap();
        let folder = root.path().join("jobs");
        let recorder = Recorder::begin(Ticket::new(&folder, 1, 1), &spec(root.path())).unwrap();
        assert!(
            left_in(&folder, &groups()).journals.is_empty(),
            "its lock is held: the job is alive"
        );
        recorder.abandon();
        let left = left_in(&folder, &groups());
        assert_eq!(left.journals.len(), 1, "{left:?}");
        assert!(left.problems.is_empty());
    }

    #[test]
    fn a_corrupt_journal_is_a_problem_to_report_and_never_a_stop() {
        let root = tempfile::tempdir().unwrap();
        let folder = root.path().join("jobs");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("1-1.journal"), b"\xff\xfe not text").unwrap();
        std::fs::write(
            folder.join("1-2.journal"),
            "fcmd-journal 1\nkind sideways\n",
        )
        .unwrap();
        std::fs::write(folder.join("notes.txt"), "not ours to read").unwrap();
        let left = left_in(&folder, &groups());
        assert!(left.journals.is_empty());
        assert_eq!(left.problems.len(), 2, "{left:?}");
        assert_eq!(left.problems[0].1, JournalError::Foreign);
        // And a folder that is not there holds nothing, quietly.
        assert_eq!(
            left_in(&root.path().join("nowhere"), &groups()),
            Left::default()
        );
    }

    #[test]
    fn discarding_takes_the_journal_and_the_parts_it_names_and_nothing_else() {
        let root = tempfile::tempdir().unwrap();
        let folder = root.path().join("jobs");
        let dst = root.path().join("dst");
        std::fs::create_dir_all(&dst).unwrap();
        std::fs::write(dst.join("whole.txt"), b"finished").unwrap();
        std::fs::write(dst.join("half.bin.fcmd-part"), b"hal").unwrap();
        std::fs::write(dst.join("other.fcmd-part"), b"somebody else's").unwrap();

        let mut recorder = Recorder::begin(Ticket::new(&folder, 1, 1), &spec(root.path()))
            .unwrap()
            .eagerly();
        recorder.opened(&VPath::local(dst.join("whole.txt")));
        recorder.closed(&VPath::local(dst.join("whole.txt")));
        recorder.opened(&VPath::local(dst.join("half.bin")));
        recorder.abandon();

        let mut left = left_in(&folder, &groups());
        let (path, journal) = left.journals.remove(0);
        discard(&crate::LocalFs, &path, &journal);
        assert!(!folder.exists(), "the journal and its folder are gone");
        assert!(!dst.join("half.bin.fcmd-part").exists());
        assert!(dst.join("whole.txt").exists(), "a finished file stays");
        assert!(
            dst.join("other.fcmd-part").exists(),
            "a part the journal does not name is not this job's to delete"
        );
    }

    #[test]
    fn the_journal_is_not_rewritten_for_every_file() {
        let root = tempfile::tempdir().unwrap();
        let folder = root.path().join("jobs");
        let ticket = Ticket::new(&folder, 1, 1);
        let path = ticket.path.clone();
        let mut recorder = Recorder::begin(ticket, &spec(root.path())).unwrap();
        let dst = VPath::local(root.path().join("dst"));
        for n in 0..50 {
            let file = dst.join(format!("{n}.txt"));
            recorder.opened(&file);
            recorder.closed(&file);
        }
        let on_disk = Journal::read(&path, &groups()).unwrap();
        assert!(
            on_disk.done.len() < 50,
            "fifty files in a moment were fifty writes"
        );
        assert_eq!(recorder.journal().done.len(), 50);
        recorder.finish();
    }
}
