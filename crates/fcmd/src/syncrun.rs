//! What a synchronisation did, written down (SPEC 03 §10a).
//!
//! A synchronisation is up to four ordinary jobs, one after the other, and each of them says
//! what it did on the status line and is gone. That is enough for somebody watching; it is
//! not enough for a backup run every evening, where the question a week later is what went
//! across on Tuesday. So a run that was given a file to write to keeps, beside the jobs, the
//! rows each of them was made of, hears how each one ended, and at the end writes one text
//! file: what was copied each way, what was taken out, what was left alone though it
//! differed, and what failed.
//!
//! This module is the bookkeeping and the text. It runs nothing and writes nothing: the `App`
//! tells it how each step ended and hands the text to a worker.

use fcmd_core::VPath;
use fcmd_i18n::I18n;
use fcmd_jobs::sync::at;
use fcmd_jobs::Outcome;

/// Which of the four jobs a step is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepKind {
    /// Copy from the left side to the right.
    ToRight,
    /// Copy from the right side to the left.
    ToLeft,
    /// Take out of the right side.
    CutRight,
    /// Take out of the left side.
    CutLeft,
}

/// One job of the run: the rows it carries, and how it ended once it has.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub kind: StepKind,
    /// The rows of the table, as paths relative to the two roots.
    pub rels: Vec<String>,
    /// `None` until the job ends — and for good, when an earlier step failed and this one
    /// was never started.
    pub outcome: Option<Outcome>,
}

/// A synchronisation that is running and will be written up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncRun {
    /// The job of the step that is running now.
    pub job: u64,
    /// The name it was saved under, or empty.
    pub name: String,
    pub left: VPath,
    pub right: VPath,
    /// Where the report goes.
    pub report: VPath,
    pub steps: Vec<Step>,
    /// Which step is running.
    pub at: usize,
    /// The rows that differ and were left as they are: answered `=` in the table, or a clash
    /// nobody resolved.
    pub left_alone: Vec<String>,
}

impl SyncRun {
    /// The step that is running has ended.
    pub fn ended(&mut self, outcome: Outcome) {
        if let Some(step) = self.steps.get_mut(self.at) {
            step.outcome = Some(outcome);
        }
    }

    /// The report, as the text of the file.
    ///
    /// `when` is the moment it is written, already spelled out: the caller has the clock, and
    /// a test has to be able to say what the second line is.
    pub fn text(&self, when: &str, i18n: &I18n) -> String {
        let mut out = String::new();
        let mut line = |text: String| {
            out.push_str(&text);
            out.push('\n');
        };
        line(i18n.t("sync-report-title"));
        line(when.to_string());
        if !self.name.is_empty() {
            line(i18n.t_args("sync-report-name", &[("name", self.name.clone().into())]));
        }
        line(i18n.t_args(
            "sync-report-left",
            &[("path", self.left.to_string().into())],
        ));
        line(i18n.t_args(
            "sync-report-right",
            &[("path", self.right.to_string().into())],
        ));

        let mut done: [Vec<&str>; 4] = Default::default();
        let mut stopped: Vec<&str> = Vec::new();
        let mut not_done: Vec<&str> = Vec::new();
        let mut failed: Vec<String> = Vec::new();
        for step in &self.steps {
            let Some(outcome) = &step.outcome else {
                not_done.extend(step.rels.iter().map(String::as_str));
                continue;
            };
            // An error names the file it happened to, which for a row that is a directory is
            // somewhere under it: the row failed if anything of it did.
            let root = match step.kind {
                StepKind::ToRight | StepKind::CutRight => &self.right,
                StepKind::ToLeft | StepKind::CutLeft => &self.left,
            };
            failed.extend(
                outcome
                    .errors
                    .iter()
                    .map(|(path, error)| format!("{path}: {error}")),
            );
            for rel in &step.rels {
                let place = at(root, rel);
                let broke = outcome
                    .errors
                    .iter()
                    .any(|(path, _)| *path == place || path.starts_with(&place));
                if broke {
                    continue;
                }
                // A step that was stopped does not say which of its rows it got to, and a
                // report that called them all done would be a report that lies.
                if outcome.cancelled {
                    stopped.push(rel);
                } else {
                    done[step.kind as usize].push(rel);
                }
            }
        }

        let mut section = |id: &str, rows: &[&str], always: bool| {
            if rows.is_empty() && !always {
                return;
            }
            line(String::new());
            line(i18n.t_args(id, &[("count", (rows.len() as i64).into())]));
            for row in rows {
                line(format!("  {row}"));
            }
        };
        section(
            "sync-report-to-right",
            &done[StepKind::ToRight as usize],
            true,
        );
        section(
            "sync-report-to-left",
            &done[StepKind::ToLeft as usize],
            true,
        );
        section(
            "sync-report-cut-right",
            &done[StepKind::CutRight as usize],
            true,
        );
        section(
            "sync-report-cut-left",
            &done[StepKind::CutLeft as usize],
            true,
        );
        let alone: Vec<&str> = self.left_alone.iter().map(String::as_str).collect();
        section("sync-report-skipped", &alone, true);
        let failed: Vec<&str> = failed.iter().map(String::as_str).collect();
        section("sync-report-failed", &failed, true);
        section("sync-report-stopped", &stopped, false);
        section("sync-report-not-done", &not_done, false);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(root: &std::path::Path) -> SyncRun {
        let step = |kind, rels: &[&str]| Step {
            kind,
            rels: rels.iter().map(|r| r.to_string()).collect(),
            outcome: None,
        };
        SyncRun {
            job: 1,
            name: "Evening backup".to_string(),
            left: VPath::local(root.join("work")),
            right: VPath::local(root.join("backup")),
            report: VPath::local(root.join("report.txt")),
            steps: vec![
                step(StepKind::ToRight, &["new.txt", "docs", "broken.bin"]),
                step(StepKind::ToLeft, &["theirs.txt"]),
                step(StepKind::CutRight, &["old.txt"]),
            ],
            at: 0,
            left_alone: vec!["clash.txt".to_string()],
        }
    }

    /// The lines under a heading, up to the next blank line.
    fn under<'a>(text: &'a str, heading: &str) -> Vec<&'a str> {
        text.lines()
            .skip_while(|l| !l.starts_with(heading))
            .skip(1)
            .take_while(|l| !l.is_empty())
            .map(str::trim)
            .collect()
    }

    #[test]
    fn a_report_says_what_went_each_way_what_was_left_and_what_failed() {
        let root = tempfile::tempdir().unwrap();
        let mut run = run(root.path());
        // The copy to the right: one file inside `docs` could not be written.
        let inside = run.right.join("docs").join("locked.txt");
        run.ended(Outcome {
            files: 2,
            errors: vec![
                (inside.clone(), "permission denied".to_string()),
                (run.right.join("broken.bin"), "disk full".to_string()),
            ],
            ..Outcome::default()
        });
        run.at = 1;
        run.ended(Outcome {
            files: 1,
            ..Outcome::default()
        });
        run.at = 2;
        run.ended(Outcome {
            files: 1,
            ..Outcome::default()
        });

        let text = run.text("2026-11-11 21:30:00", &I18n::new("en"));
        let mut lines = text.lines();
        assert_eq!(lines.next(), Some("Synchronisation report"));
        assert_eq!(lines.next(), Some("2026-11-11 21:30:00"));
        assert_eq!(lines.next(), Some("Name: Evening backup"));
        assert!(text.contains(&run.left.to_string()), "{text}");
        assert!(text.contains(&run.right.to_string()), "{text}");

        assert_eq!(
            under(&text, "Copied left → right: 1"),
            ["new.txt"],
            "a row with an error under it is not a row that was copied:\n{text}"
        );
        assert_eq!(under(&text, "Copied right → left: 1"), ["theirs.txt"]);
        assert_eq!(under(&text, "Taken out on the right: 1"), ["old.txt"]);
        assert!(text.contains("Taken out on the left: 0"), "{text}");
        assert_eq!(
            under(&text, "Left alone though they differ: 1"),
            ["clash.txt"]
        );
        let failed = under(&text, "Failed: 2");
        assert_eq!(failed.len(), 2, "{text}");
        assert_eq!(failed[0], format!("{inside}: permission denied"));
    }

    #[test]
    fn a_step_that_never_ran_and_one_that_was_stopped_are_not_called_done() {
        let root = tempfile::tempdir().unwrap();
        let mut run = run(root.path());
        // `Esc` halfway through the first copy: the two after it never start.
        run.ended(Outcome {
            files: 1,
            cancelled: true,
            ..Outcome::default()
        });
        let text = run.text("now", &I18n::new("en"));
        assert!(text.contains("Copied left → right: 0"), "{text}");
        assert_eq!(
            under(&text, "Stopped part of the way").len(),
            3,
            "every row of the stopped step is in doubt:\n{text}"
        );
        assert_eq!(
            under(&text, "Not done"),
            ["theirs.txt", "old.txt"],
            "{text}"
        );
    }

    #[test]
    fn a_run_with_no_name_and_nothing_wrong_says_so_briefly() {
        let root = tempfile::tempdir().unwrap();
        let mut run = run(root.path());
        run.name.clear();
        run.left_alone.clear();
        run.steps.truncate(1);
        run.ended(Outcome::default());
        let text = run.text("now", &I18n::new("en"));
        assert!(!text.contains("Name:"), "{text}");
        assert!(text.contains("Failed: 0"), "{text}");
        assert!(!text.contains("Stopped"), "{text}");
        assert!(!text.contains("Not done"), "{text}");
    }
}
