//! The folders one actually uses, most used and most lately first (SPEC 02 §11b).
//!
//! The folder history is where a panel has been, in the order it went there; the shortcuts
//! are the folders somebody chose to keep. This is the third thing, which nobody keeps and
//! everybody wants: the dozen folders the work happens in, found by typing three letters of
//! one — because they were visited often, or just now, or both.
//!
//! Every visit is counted and stamped. A folder's rank is its count weighed by how long ago
//! the last visit was, in the four steps zoxide made familiar: within the hour counts four
//! times, within the day twice, within the week half, and older than that a quarter. So a
//! folder opened every day for a year outranks one opened ten times this morning only until
//! tomorrow, and a project finished last spring sinks without anybody tidying.

/// How many folders are remembered. Past this the lowest-ranked is forgotten, which is the
/// only housekeeping there is: a folder nobody goes back to falls off the end by itself.
pub const KEPT: usize = 500;

const HOUR: i64 = 3_600;
const DAY: i64 = 86_400;
const WEEK: i64 = 7 * DAY;

/// One folder: how often, and when last.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Visit {
    pub path: String,
    pub count: u32,
    /// Seconds since the epoch.
    pub last: i64,
}

impl Visit {
    /// The rank at `now`: higher is nearer the top. Whole numbers, so two machines and two
    /// runs order the same list the same way.
    pub fn score(&self, now: i64) -> u64 {
        let age = now.saturating_sub(self.last);
        // In quarters, so that the smallest weight is still a whole number.
        let weight = if age < HOUR {
            16
        } else if age < DAY {
            8
        } else if age < WEEK {
            2
        } else {
            1
        };
        u64::from(self.count) * weight
    }
}

/// Every folder visited, in no particular order: the order is asked for with a clock.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Visits {
    rows: Vec<Visit>,
}

impl Visits {
    pub fn from_state(state: &[fcmd_config::FolderVisit]) -> Visits {
        Visits {
            rows: state
                .iter()
                .filter(|v| !v.path.is_empty())
                .map(|v| Visit {
                    path: v.path.clone(),
                    count: v.count.max(1),
                    last: v.last,
                })
                .collect(),
        }
    }

    /// What the state file keeps: best first as of `now`, so a file read by eye says the
    /// same thing the list does.
    pub fn to_state(&self, now: i64) -> Vec<fcmd_config::FolderVisit> {
        self.ranked(now)
            .into_iter()
            .map(|v| fcmd_config::FolderVisit {
                path: v.path.clone(),
                count: v.count,
                last: v.last,
            })
            .collect()
    }

    /// How many folders are held, which only the tests of the cap ask.
    #[cfg(test)]
    fn len(&self) -> usize {
        self.rows.len()
    }

    /// A panel arrived in a folder.
    pub fn visit(&mut self, path: &str, now: i64) {
        if path.is_empty() {
            return;
        }
        match self.rows.iter_mut().find(|v| v.path == path) {
            Some(visit) => {
                visit.count = visit.count.saturating_add(1);
                visit.last = now;
            }
            None => self.rows.push(Visit {
                path: path.to_string(),
                count: 1,
                last: now,
            }),
        }
        // One over, one out: the folder that ranks lowest now, and of two that rank the
        // same the one visited longer ago — never the one that has just come in.
        while self.rows.len() > KEPT {
            let Some(worst) = self
                .rows
                .iter()
                .enumerate()
                .filter(|(_, v)| v.path != path)
                .min_by_key(|(_, v)| (v.score(now), v.last))
                .map(|(i, _)| i)
            else {
                break;
            };
            self.rows.swap_remove(worst);
        }
    }

    /// Take a folder out — one that is gone, or one that was only ever a mistake.
    pub fn forget(&mut self, path: &str) -> bool {
        let before = self.rows.len();
        self.rows.retain(|v| v.path != path);
        self.rows.len() != before
    }

    /// Best first. Of two that rank the same, the one visited last; of two visited in the
    /// same second, by name, so the order never depends on how the list happens to be kept.
    pub fn ranked(&self, now: i64) -> Vec<&Visit> {
        let mut rows: Vec<&Visit> = self.rows.iter().collect();
        rows.sort_by(|a, b| {
            b.score(now)
                .cmp(&a.score(now))
                .then(b.last.cmp(&a.last))
                .then(a.path.cmp(&b.path))
        });
        rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_800_000_000;

    fn paths(visits: &Visits, now: i64) -> Vec<&str> {
        visits
            .ranked(now)
            .into_iter()
            .map(|v| v.path.as_str())
            .collect()
    }

    #[test]
    fn a_visit_is_counted_and_stamped() {
        let mut visits = Visits::default();
        visits.visit("/src", NOW - 10);
        visits.visit("/src", NOW);
        visits.visit("", NOW);
        assert_eq!(visits.len(), 1);
        let ranked = visits.ranked(NOW);
        assert_eq!(ranked[0].count, 2);
        assert_eq!(ranked[0].last, NOW);
    }

    #[test]
    fn often_and_lately_both_count_and_lately_fades() {
        let mut visits = Visits::default();
        // Ten times, a month ago; twice, this hour; three times, yesterday.
        for _ in 0..10 {
            visits.visit("/old-project", NOW - 30 * DAY);
        }
        for _ in 0..2 {
            visits.visit("/today", NOW - 60);
        }
        for _ in 0..3 {
            visits.visit("/yesterday", NOW - 20 * HOUR);
        }
        // 2 × 16, 3 × 8, 10 × 1.
        assert_eq!(
            paths(&visits, NOW),
            ["/today", "/yesterday", "/old-project"]
        );
        // A week on with nothing visited, the count is all that is left to tell them apart.
        assert_eq!(
            paths(&visits, NOW + 8 * DAY),
            ["/old-project", "/yesterday", "/today"]
        );
    }

    #[test]
    fn equal_ranks_go_by_the_last_visit_and_then_by_name() {
        let mut visits = Visits::default();
        visits.visit("/b", NOW - 30);
        visits.visit("/a", NOW - 30);
        visits.visit("/c", NOW - 10);
        assert_eq!(paths(&visits, NOW), ["/c", "/a", "/b"]);
    }

    #[test]
    fn a_folder_can_be_forgotten() {
        let mut visits = Visits::default();
        visits.visit("/gone", NOW);
        visits.visit("/here", NOW);
        assert!(visits.forget("/gone"));
        assert!(!visits.forget("/gone"));
        assert_eq!(paths(&visits, NOW), ["/here"]);
    }

    #[test]
    fn the_list_is_capped_and_the_lowest_rank_is_what_goes() {
        let mut visits = Visits::default();
        // One that matters, long ago but often; then more one-off folders than are kept.
        for _ in 0..50 {
            visits.visit("/work", NOW - 400 * DAY);
        }
        for i in 0..KEPT + 20 {
            visits.visit(&format!("/tmp/once-{i}"), NOW - 200 * DAY + i as i64);
        }
        assert_eq!(visits.len(), KEPT);
        let kept = paths(&visits, NOW);
        assert_eq!(kept[0], "/work");
        // The oldest of the one-offs went; the newest is still there, and so is the one
        // that came in last even though nothing ranks lower than it.
        assert!(!kept.contains(&"/tmp/once-0"));
        assert!(kept.contains(&format!("/tmp/once-{}", KEPT + 19).as_str()));
    }

    #[test]
    fn the_state_file_holds_it_best_first_and_gives_it_back() {
        let mut visits = Visits::default();
        visits.visit("/a", NOW - 30 * DAY);
        visits.visit("/b", NOW);
        let state = visits.to_state(NOW);
        assert_eq!(state[0].path, "/b");
        assert_eq!(state[1].path, "/a");
        let back = Visits::from_state(&state);
        assert_eq!(paths(&back, NOW), ["/b", "/a"]);
        // A row somebody emptied by hand is nothing, and a count of none is one visit.
        let odd = Visits::from_state(&[
            fcmd_config::FolderVisit {
                path: String::new(),
                count: 3,
                last: NOW,
            },
            fcmd_config::FolderVisit {
                path: "/x".into(),
                count: 0,
                last: NOW,
            },
        ]);
        assert_eq!(odd.len(), 1);
        assert_eq!(odd.ranked(NOW)[0].count, 1);
    }
}
