//! The jobs, and which of them may run (SPEC 03 §5).
//!
//! Every copy, move, delete and the rest of what goes through `App::start_job` is a row here,
//! from the moment it is asked for until well after it ends: running, waiting for its turn, or
//! finished with what it did. What decides the turns is the *lane*: by default one per target
//! device, so two copies onto the same disk run one after the other, which is quicker than
//! both at once fighting over one head, and a copy onto a USB stick does not wait for one onto
//! the network. `general.job_queue` makes it one lane for everything, or no waiting at all.
//!
//! This module is the bookkeeping, with no screen and no thread in it: which job is next,
//! which lane is free, moving a row up the list. The `App` owns the queue and does the
//! starting; the tests here can therefore say what the order is without running anything.

use std::collections::VecDeque;

use fcmd_config::JobQueue;
use fcmd_core::{CancelToken, VPath};
use fcmd_jobs::{JobKind, JobSpec, Outcome, Progress};

use crate::app::{Carry, PendingQuestion};

/// Where a job is in its life. A paused job is still `Running` — it holds its lane and its
/// thread, and only its token says it is waiting — and a paused queued job is `Queued` with
/// [`Job::paused`] set, which means "do not start it yet".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    Queued,
    Running,
    Done,
}

#[derive(Debug)]
pub struct Job {
    pub id: u64,
    pub kind: JobKind,
    /// What it works on, kept whole: a queued job is started from it, and a finished one is
    /// retried from it.
    pub spec: JobSpec,
    pub lane: String,
    pub state: JobState,
    pub paused: bool,
    pub progress: Progress,
    pub cancel: CancelToken,
    /// A question the engine is blocked on while nobody is looking at this job.
    pub question: Option<PendingQuestion>,
    pub outcome: Option<Outcome>,
    /// What it will do to the descriptions, worked out when it was asked for: by the time it
    /// ends the panels may be somewhere else.
    pub carry: Option<Carry>,
    /// How to undo it, worked out when it was asked for, and kept once it ends well.
    pub undo: Option<crate::undo::Undo>,
    /// The jobs that follow this one: the next destinations of a copy to several places, the
    /// rest of a synchronisation. They start when this ends well and are dropped when it does
    /// not.
    pub then: VecDeque<JobSpec>,
}

impl Job {
    pub fn new(id: u64, spec: JobSpec, lane: String) -> Job {
        Job {
            id,
            kind: spec.kind,
            spec,
            lane,
            state: JobState::Queued,
            paused: false,
            progress: Progress::default(),
            cancel: CancelToken::new(),
            question: None,
            outcome: None,
            carry: None,
            undo: None,
            then: VecDeque::new(),
        }
    }

    /// Ended with errors, or stopped: a job worth retrying.
    pub fn failed(&self) -> bool {
        self.outcome
            .as_ref()
            .is_some_and(|o| o.cancelled || !o.errors.is_empty())
    }
}

/// Finished jobs kept in the list for their results. Older ones go, oldest first.
const KEEP_DONE: usize = 20;

#[derive(Debug, Default)]
pub struct Queue {
    pub jobs: Vec<Job>,
}

impl Queue {
    pub fn get(&self, id: u64) -> Option<&Job> {
        self.jobs.iter().find(|j| j.id == id)
    }

    pub fn get_mut(&mut self, id: u64) -> Option<&mut Job> {
        self.jobs.iter_mut().find(|j| j.id == id)
    }

    /// Is something running on this lane? A paused job counts: it is halfway through.
    pub fn lane_busy(&self, lane: &str, policy: JobQueue) -> bool {
        match policy {
            JobQueue::Off => false,
            JobQueue::One => self.jobs.iter().any(|j| j.state == JobState::Running),
            JobQueue::Device => self
                .jobs
                .iter()
                .any(|j| j.state == JobState::Running && j.lane == lane),
        }
    }

    /// The jobs to start now, in the order of the list: every queued job not held back whose
    /// lane is free, at most one per lane.
    pub fn startable(&self, policy: JobQueue) -> Vec<u64> {
        let mut taken: Vec<&str> = Vec::new();
        let mut any = self.jobs.iter().any(|j| j.state == JobState::Running);
        let mut out = Vec::new();
        for job in &self.jobs {
            if job.state != JobState::Queued || job.paused {
                continue;
            }
            let free = match policy {
                JobQueue::Off => true,
                JobQueue::One => !any,
                JobQueue::Device => {
                    !taken.contains(&job.lane.as_str()) && !self.lane_busy(&job.lane, policy)
                }
            };
            if free {
                out.push(job.id);
                taken.push(&job.lane);
                any = true;
            }
        }
        out
    }

    /// Jobs not yet finished: what quitting would stop.
    pub fn active(&self) -> usize {
        self.jobs
            .iter()
            .filter(|j| j.state != JobState::Done)
            .count()
    }

    /// Move a job one row up or down among the ones still waiting. Only the order of the
    /// waiting ones means anything: a running job has started and a finished one has ended.
    pub fn shift(&mut self, id: u64, up: bool) -> bool {
        let Some(at) = self.jobs.iter().position(|j| j.id == id) else {
            return false;
        };
        if self.jobs[at].state != JobState::Queued {
            return false;
        }
        let other = if up {
            (0..at)
                .rev()
                .find(|&i| self.jobs[i].state == JobState::Queued)
        } else {
            (at + 1..self.jobs.len()).find(|&i| self.jobs[i].state == JobState::Queued)
        };
        match other {
            Some(other) => {
                self.jobs.swap(at, other);
                true
            }
            None => false,
        }
    }

    /// Forget the oldest finished jobs beyond the few kept for their results.
    pub fn prune(&mut self) {
        let mut done = self
            .jobs
            .iter()
            .filter(|j| j.state == JobState::Done)
            .count();
        self.jobs.retain(|j| {
            if j.state == JobState::Done && done > KEEP_DONE {
                done -= 1;
                false
            } else {
                true
            }
        });
    }
}

/// The lane a job waits in: the device its target is on, or its sources' for a job with no
/// target (a delete).
///
/// A device is the filesystem the path is on — `st_dev` on Unix, the drive or share on
/// Windows — and a server for a remote path. A path inside an archive is on the device of the
/// archive's file. A path that does not exist yet is on the device of its nearest ancestor
/// that does, which is where it will be created.
pub fn lane(spec: &JobSpec) -> String {
    let path = spec
        .target
        .clone()
        .or_else(|| spec.sources.first().and_then(|p| p.parent()))
        .unwrap_or_else(|| VPath::local("."));
    lane_of(&path)
}

fn lane_of(path: &VPath) -> String {
    if !path.is_local() {
        return format!(
            "{}://{}",
            path.scheme().as_str(),
            path.authority().unwrap_or_default()
        );
    }
    let os = path.os().into_owned();
    device(&os)
}

#[cfg(unix)]
fn device(path: &std::path::Path) -> String {
    use std::os::unix::fs::MetadataExt;
    let mut at = Some(path);
    while let Some(p) = at {
        if let Ok(meta) = std::fs::metadata(p) {
            return format!("dev:{}", meta.dev());
        }
        at = p.parent();
    }
    "dev:?".to_string()
}

#[cfg(windows)]
fn device(path: &std::path::Path) -> String {
    use std::path::Component;
    match path.components().next() {
        Some(Component::Prefix(prefix)) => {
            format!(
                "dev:{}",
                prefix.as_os_str().to_string_lossy().to_uppercase()
            )
        }
        _ => std::env::current_dir()
            .ok()
            .and_then(|cwd| match cwd.components().next() {
                Some(Component::Prefix(p)) => Some(format!(
                    "dev:{}",
                    p.as_os_str().to_string_lossy().to_uppercase()
                )),
                _ => None,
            })
            .unwrap_or_else(|| "dev:?".to_string()),
    }
}

#[cfg(not(any(unix, windows)))]
fn device(_: &std::path::Path) -> String {
    "dev:?".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(id: u64, lane: &str, state: JobState) -> Job {
        let mut j = Job::new(
            id,
            JobSpec::new(JobKind::Copy, vec![VPath::local("/a")], None),
            lane.to_string(),
        );
        j.state = state;
        j
    }

    fn queue(jobs: Vec<Job>) -> Queue {
        Queue { jobs }
    }

    #[test]
    fn one_job_per_device_and_the_rest_wait() {
        let q = queue(vec![
            job(1, "disk", JobState::Running),
            job(2, "disk", JobState::Queued),
            job(3, "usb", JobState::Queued),
            job(4, "usb", JobState::Queued),
        ]);
        // The disk is busy; the stick is free, and takes only the first of its two.
        assert_eq!(q.startable(JobQueue::Device), vec![3]);
        // One lane for everything: nothing starts while anything runs.
        assert!(q.startable(JobQueue::One).is_empty());
        // No queue at all: everything waiting starts.
        assert_eq!(q.startable(JobQueue::Off), vec![2, 3, 4]);
    }

    #[test]
    fn a_held_job_lets_the_one_behind_it_go_first() {
        let mut q = queue(vec![
            job(1, "disk", JobState::Queued),
            job(2, "disk", JobState::Queued),
        ]);
        q.jobs[0].paused = true;
        assert_eq!(q.startable(JobQueue::Device), vec![2]);
    }

    #[test]
    fn a_paused_running_job_still_holds_its_lane() {
        let mut q = queue(vec![
            job(1, "disk", JobState::Running),
            job(2, "disk", JobState::Queued),
        ]);
        q.jobs[0].paused = true;
        assert!(q.startable(JobQueue::Device).is_empty());
    }

    #[test]
    fn only_waiting_jobs_change_places() {
        let mut q = queue(vec![
            job(1, "disk", JobState::Running),
            job(2, "disk", JobState::Queued),
            job(3, "disk", JobState::Done),
            job(4, "disk", JobState::Queued),
        ]);
        // 4 moves up past the finished one, to where 2 was.
        assert!(q.shift(4, true));
        let order: Vec<u64> = q.jobs.iter().map(|j| j.id).collect();
        assert_eq!(order, vec![1, 4, 3, 2]);
        assert_eq!(q.startable(JobQueue::Off), vec![4, 2]);
        // Nothing waits above it any more, and the running one does not move.
        assert!(!q.shift(4, true));
        assert!(!q.shift(1, false));
    }

    #[test]
    fn old_results_are_forgotten_and_live_jobs_never() {
        let mut jobs: Vec<Job> = (0..30).map(|i| job(i, "d", JobState::Done)).collect();
        jobs.push(job(99, "d", JobState::Queued));
        let mut q = queue(jobs);
        q.prune();
        assert_eq!(q.jobs.len(), KEEP_DONE + 1);
        assert_eq!(q.jobs[0].id, 10, "the oldest go first");
        assert!(q.get(99).is_some());
    }

    #[test]
    fn a_target_that_does_not_exist_yet_is_on_its_parents_device() {
        let dir = tempfile::tempdir().unwrap();
        let there = VPath::local(dir.path());
        let later = VPath::local(dir.path().join("not/yet/made"));
        let a = lane(&JobSpec::new(JobKind::Copy, vec![], Some(there)));
        let b = lane(&JobSpec::new(JobKind::Copy, vec![], Some(later)));
        assert_eq!(a, b);
        assert!(a.starts_with("dev:"));
    }

    #[test]
    fn a_server_is_a_lane_of_its_own_and_a_delete_goes_by_its_sources() {
        let sftp = VPath::parse("sftp://me@host/home/me");
        assert_eq!(
            lane(&JobSpec::new(JobKind::Copy, vec![], Some(sftp))),
            "sftp://me@host"
        );
        let gone = VPath::parse("sftp://me@host/home/me/old.txt");
        assert_eq!(
            lane(&JobSpec::new(JobKind::Delete, vec![gone], None)),
            "sftp://me@host"
        );
    }
}
