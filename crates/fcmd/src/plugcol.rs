//! The columns a plugin computes, as the worker sees them (SPEC 08 §1d).
//!
//! A `[[column]]` of a plugin names a program; this is the half that runs it, away from the
//! interface. The application decides *which* files want an answer — the rows of one
//! directory that match the column's masks and are not in the cache already — and hands
//! them over as one [`Job`]; what comes back is a run of [`Answer`]s, a few rows at a time,
//! so that a directory of a thousand pictures fills in from the top instead of standing
//! empty until the last one is done.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

/// A column a plugin provides, with everything needed to ask it resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginColumn {
    /// Which plugin it came from, and what the plugin calls it: a `[[column_set]]` names
    /// the two together, `plugin:<plugin>.<id>`.
    pub plugin: String,
    pub id: String,
    /// The heading, in the plugin's own words.
    pub title: String,
    /// The width the plugin asks for when the set does not say.
    pub width: Option<u16>,
    pub right: bool,
    pub program: PathBuf,
    /// The files it has an answer for; `None` is every file.
    pub masks: Option<String>,
    pub timeout: Option<u64>,
    /// One run for a directory rather than one per file.
    pub batch: bool,
    /// Directories are asked about as well.
    pub folders: bool,
}

impl PluginColumn {
    /// What a column set writes after `type =`.
    pub fn name(&self) -> String {
        format!("plugin:{}.{}", self.plugin, self.id)
    }
}

/// One file a column is asked about. The date and the size travel with the name because
/// they are what the answer is kept under: a file that changes is asked about again, and
/// one that has not is never asked about twice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asked {
    pub name: String,
    pub mtime: Option<SystemTime>,
    pub size: u64,
}

/// One column's work for one directory.
#[derive(Debug, Clone)]
pub struct Job {
    /// The column's own id, which the program finds in `FCMD_COLUMN`: a plugin may serve
    /// several columns from one script.
    pub id: String,
    pub program: PathBuf,
    pub dir: PathBuf,
    pub files: Vec<Asked>,
    pub batch: bool,
    pub timeout: Option<u64>,
    /// Set when the panel has moved on: the rows this was for are no longer on screen.
    pub cancel: Arc<AtomicBool>,
}

/// Two jobs are the same job when they ask the same thing for the same listing — which is
/// to say when they would be stopped by the same flag.
impl PartialEq for Job {
    fn eq(&self, other: &Job) -> bool {
        self.id == other.id
            && self.program == other.program
            && self.dir == other.dir
            && self.files == other.files
            && self.batch == other.batch
            && self.timeout == other.timeout
            && Arc::ptr_eq(&self.cancel, &other.cancel)
    }
}

/// A part of the answer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Answer {
    /// What the program said about these files — worth keeping, since asking again would
    /// say the same.
    pub cells: Vec<(Asked, String)>,
    /// The files it was asked about and gave no answer for: a run that timed out, or what
    /// was left when the column gave up. Their cells go empty and nothing is remembered,
    /// so the next reading of the directory asks again.
    pub blank: Vec<String>,
    /// Why the column gave up, when it did.
    pub error: Option<String>,
}

/// How many runs may time out before the column stops asking for this directory. Each one
/// has already cost its whole timeout, and a program that hung on three files in a row is
/// not going to answer for the four hundred after them.
const TIMEOUTS_BEFORE_GIVING_UP: usize = 3;

/// Rows handed over at a time, and how long a row waits for company at most.
const CHUNK: usize = 16;
const CHUNK_AGE: Duration = Duration::from_millis(150);

/// Do a job, handing the answers to `emit` as they come.
pub fn run(job: &Job, emit: &mut dyn FnMut(Answer)) {
    use fcmd_plug::host::{column_batch, column_value, RunError};

    if job.files.is_empty() {
        return;
    }
    if job.batch {
        let paths: Vec<PathBuf> = job.files.iter().map(|f| job.dir.join(&f.name)).collect();
        let answer = match column_batch(&job.program, &job.dir, &paths, &job.id, job.timeout) {
            Ok(values) => Answer {
                cells: job.files.iter().cloned().zip(values).collect(),
                ..Answer::default()
            },
            Err(e) => Answer {
                blank: job.files.iter().map(|f| f.name.clone()).collect(),
                error: Some(e.to_string()),
                ..Answer::default()
            },
        };
        emit(answer);
        return;
    }

    let mut pending = Answer::default();
    let mut since = Instant::now();
    let mut timeouts = 0;
    for (n, file) in job.files.iter().enumerate() {
        if job.cancel.load(Ordering::Relaxed) {
            // Nobody is looking any more. What is already known is still true of those
            // files, so it is handed over to be remembered; the rest is simply not asked.
            break;
        }
        let path = job.dir.join(&file.name);
        match column_value(&job.program, &job.dir, &path, &job.id, job.timeout) {
            Ok(value) => pending.cells.push((file.clone(), value)),
            // A program that ends with a failing status has said that it has nothing to
            // say about this file — `grep` does exactly that — and it would say so again.
            Err(RunError::Status { .. }) => pending.cells.push((file.clone(), String::new())),
            Err(e @ RunError::Timeout { .. }) => {
                timeouts += 1;
                pending.blank.push(file.name.clone());
                if timeouts >= TIMEOUTS_BEFORE_GIVING_UP {
                    pending
                        .blank
                        .extend(job.files[n + 1..].iter().map(|f| f.name.clone()));
                    pending.error = Some(e.to_string());
                    break;
                }
            }
            // It could not be started at all, and the next file would find the same.
            Err(e) => {
                pending
                    .blank
                    .extend(job.files[n..].iter().map(|f| f.name.clone()));
                pending.error = Some(e.to_string());
                break;
            }
        }
        if pending.cells.len() + pending.blank.len() >= CHUNK || since.elapsed() >= CHUNK_AGE {
            emit(std::mem::take(&mut pending));
            since = Instant::now();
        }
    }
    if pending != Answer::default() {
        emit(pending);
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    /// A script in `dir`, or `None` on a machine with no shell to run it.
    fn script(dir: &std::path::Path, name: &str, body: &str) -> Option<PathBuf> {
        use std::os::unix::fs::PermissionsExt;
        if !std::path::Path::new("/bin/sh").exists() {
            return None;
        }
        let path = dir.join(name);
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        Some(path)
    }

    fn asked(names: &[&str]) -> Vec<Asked> {
        names
            .iter()
            .map(|n| Asked {
                name: n.to_string(),
                mtime: None,
                size: 0,
            })
            .collect()
    }

    fn job(program: PathBuf, dir: &std::path::Path, names: &[&str], batch: bool) -> Job {
        Job {
            id: "lines".into(),
            program,
            dir: dir.to_path_buf(),
            files: asked(names),
            batch,
            timeout: Some(1),
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }

    fn all(job: &Job) -> Vec<Answer> {
        let mut out = Vec::new();
        run(job, &mut |a| out.push(a));
        out
    }

    #[test]
    fn each_file_is_asked_about_and_a_failing_status_is_an_empty_answer() {
        let plugin = tempfile::tempdir().unwrap();
        let files = tempfile::tempdir().unwrap();
        let Some(program) = script(
            plugin.path(),
            "lines.sh",
            "case \"$1\" in *.bin) exit 1;; esac\nwc -l < \"$1\"",
        ) else {
            return;
        };
        std::fs::write(files.path().join("two.txt"), "a\nb\n").unwrap();
        std::fs::write(files.path().join("blob.bin"), "x").unwrap();
        let answers = all(&job(program, files.path(), &["two.txt", "blob.bin"], false));
        let cells: Vec<(String, String)> = answers
            .iter()
            .flat_map(|a| a.cells.iter())
            .map(|(f, v)| (f.name.clone(), v.trim().to_string()))
            .collect();
        assert_eq!(
            cells,
            [
                ("two.txt".to_string(), "2".to_string()),
                ("blob.bin".to_string(), String::new())
            ]
        );
        assert!(answers
            .iter()
            .all(|a| a.error.is_none() && a.blank.is_empty()));
    }

    #[test]
    fn a_program_that_hangs_gives_the_column_up_after_three_files() {
        let plugin = tempfile::tempdir().unwrap();
        let Some(program) = script(plugin.path(), "hang.sh", "exec sleep 30") else {
            return;
        };
        let names = ["a", "b", "c", "d", "e"];
        let answers = all(&job(program, plugin.path(), &names, false));
        let blank: Vec<&str> = answers
            .iter()
            .flat_map(|a| a.blank.iter().map(String::as_str))
            .collect();
        // Three were tried; the two after them were never started, and nothing is kept.
        assert_eq!(blank, names);
        assert!(answers.iter().all(|a| a.cells.is_empty()));
        let error = answers.iter().find_map(|a| a.error.clone()).expect("said");
        assert!(error.contains("hang.sh"), "{error}");
    }

    #[test]
    fn a_program_that_cannot_start_is_said_once_and_a_cancelled_job_asks_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let gone = dir.path().join("not-there.sh");
        let answers = all(&job(gone.clone(), dir.path(), &["a", "b"], false));
        assert_eq!(answers.len(), 1);
        assert_eq!(answers[0].blank, ["a", "b"]);
        assert!(answers[0].error.is_some());
        // The same for a batch, whose one run is the whole job.
        let answers = all(&job(gone.clone(), dir.path(), &["a", "b"], true));
        assert_eq!(answers[0].blank, ["a", "b"]);
        assert!(answers[0].error.is_some());

        let cancelled = job(gone, dir.path(), &["a", "b"], false);
        cancelled.cancel.store(true, Ordering::Relaxed);
        assert!(all(&cancelled).is_empty());
    }

    #[test]
    fn a_batch_is_one_run_for_the_whole_directory() {
        let plugin = tempfile::tempdir().unwrap();
        let files = tempfile::tempdir().unwrap();
        // The script leaves a mark each time it is started, which is how one run is told
        // from three.
        let Some(program) = script(
            plugin.path(),
            "batch.sh",
            "echo run >> \"$FCMD_DIR/runs\"\nwhile IFS= read -r f; do\n  \
             printf '%s\\t%s\\n' \"$f\" \"$(basename \"$f\" | tr a-z A-Z)\"\ndone",
        ) else {
            return;
        };
        let answers = all(&job(
            program,
            files.path(),
            &["a.txt", "b.txt", "c.txt"],
            true,
        ));
        assert_eq!(answers.len(), 1);
        let values: Vec<&str> = answers[0].cells.iter().map(|(_, v)| v.as_str()).collect();
        assert_eq!(values, ["A.TXT", "B.TXT", "C.TXT"]);
        let runs = std::fs::read_to_string(files.path().join("runs")).unwrap();
        assert_eq!(runs.lines().count(), 1);
    }
}
