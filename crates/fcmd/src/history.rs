//! What the histories keep besides their lines.
//!
//! The three histories live in three places, because their lines do: the command line keeps
//! the commands, a panel keeps where it has been, the program keeps the files it has opened.
//! What belongs to all three is the same two questions — when was this line last used, and is
//! it one the user has pinned so that nothing pushes it out — and the answers live here, in
//! one table beside the three stores rather than three tables inside them. The state file
//! writes it with them (SPEC 06 §6).
//!
//! A line nobody has pinned and nobody has used for `lifetime` days is past its life and goes
//! when the program next writes its state. A pinned one never does, however old it is and
//! however full the history gets: that is the whole of what pinning means.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Which of the three histories a line belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Hist {
    Commands,
    Folders,
    Files,
}

impl Hist {
    /// What the state file calls it.
    pub fn tag(self) -> &'static str {
        match self {
            Hist::Commands => "commands",
            Hist::Folders => "folders",
            Hist::Files => "files",
        }
    }

    fn of_tag(tag: &str) -> Option<Hist> {
        match tag {
            "commands" => Some(Hist::Commands),
            "folders" => Some(Hist::Folders),
            "files" => Some(Hist::Files),
            _ => None,
        }
    }
}

/// Seconds since the epoch, which is all the stamp a lifetime in days needs.
pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// When a line was last used, and whether it is pinned.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Mark {
    when: i64,
    locked: bool,
}

/// The table: one mark per line that has ever been used or pinned.
#[derive(Debug, Default, Clone)]
pub struct Marks {
    rows: HashMap<(Hist, String), Mark>,
}

impl Marks {
    /// This line has just been used.
    ///
    /// A line that is used again is as young as the use, which is what makes a lifetime mean
    /// *idle* days rather than days since the line was first typed.
    pub fn touch(&mut self, hist: Hist, line: &str, now: i64) {
        let mark = self.rows.entry((hist, line.to_string())).or_default();
        mark.when = now;
    }

    /// Pin the line, or let it go again. The new state is what comes back.
    pub fn flip(&mut self, hist: Hist, line: &str, now: i64) -> bool {
        let mark = self.rows.entry((hist, line.to_string())).or_default();
        mark.locked = !mark.locked;
        if mark.when == 0 {
            mark.when = now;
        }
        mark.locked
    }

    pub fn is_locked(&self, hist: Hist, line: &str) -> bool {
        self.rows
            .get(&(hist, line.to_string()))
            .is_some_and(|m| m.locked)
    }

    /// Every pinned line of one history, for the trimming that must not touch them.
    pub fn locked_lines(&self, hist: Hist) -> Vec<String> {
        self.rows
            .iter()
            .filter(|((which, _), mark)| *which == hist && mark.locked)
            .map(|((_, line), _)| line.clone())
            .collect()
    }

    /// Is this line past its life: unpinned, and not used for `days` days?
    ///
    /// `days == 0` is FAR's own "keep them for ever", and a line with no stamp at all — one
    /// read from a state file written before any of this — counts as used now, so an upgrade
    /// never silently empties a history.
    pub fn is_stale(&self, hist: Hist, line: &str, now: i64, days: u32) -> bool {
        if days == 0 {
            return false;
        }
        let Some(mark) = self.rows.get(&(hist, line.to_string())) else {
            return false;
        };
        !mark.locked && mark.when > 0 && now.saturating_sub(mark.when) > days as i64 * 86_400
    }

    /// Forget one line's mark, for a line that has just left its history.
    pub fn forget(&mut self, hist: Hist, line: &str) {
        self.rows.remove(&(hist, line.to_string()));
    }

    /// Drop the marks of lines that are no longer in their history, so the table does not
    /// outgrow the thing it describes.
    pub fn keep_only(&mut self, hist: Hist, lines: &[String]) {
        self.rows
            .retain(|(which, line), _| *which != hist || lines.contains(line));
    }

    pub fn to_state(&self) -> Vec<fcmd_config::HistoryMark> {
        let mut rows: Vec<fcmd_config::HistoryMark> = self
            .rows
            .iter()
            .map(|((hist, line), mark)| fcmd_config::HistoryMark {
                kind: hist.tag().to_string(),
                text: line.clone(),
                when: mark.when,
                locked: mark.locked,
            })
            .collect();
        // A hash table has no order and a state file that reshuffles itself every run is one
        // nobody can read a diff of.
        rows.sort_by(|a, b| (&a.kind, &a.text).cmp(&(&b.kind, &b.text)));
        rows
    }

    pub fn from_state(rows: &[fcmd_config::HistoryMark]) -> Marks {
        let mut marks = Marks::default();
        for row in rows {
            let Some(hist) = Hist::of_tag(&row.kind) else {
                continue;
            };
            marks.rows.insert(
                (hist, row.text.clone()),
                Mark {
                    when: row.when,
                    locked: row.locked,
                },
            );
        }
        marks
    }
}

/// Cut a history down to `keep` lines, dropping the oldest ones that nobody has pinned.
///
/// `oldest_first` says which end of the list is the old one: the command line keeps its
/// history the way it walks it, oldest first, and the files read or edited are kept newest
/// first because that is the order their list shows. A pinned line is passed over, so a
/// history of pinned lines simply stays longer than the limit — which is what the pin asked
/// for.
pub fn trim(lines: &mut Vec<String>, keep: usize, locked: &[String], oldest_first: bool) {
    if lines.len() <= keep {
        return;
    }
    let mut over = lines.len() - keep;
    let mut doomed = vec![false; lines.len()];
    let order: Vec<usize> = if oldest_first {
        (0..lines.len()).collect()
    } else {
        (0..lines.len()).rev().collect()
    };
    for at in order {
        if over == 0 {
            break;
        }
        if locked.iter().any(|l| *l == lines[at]) {
            continue;
        }
        doomed[at] = true;
        over -= 1;
    }
    let mut at = 0;
    lines.retain(|_| {
        let gone = doomed[at];
        at += 1;
        !gone
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: i64 = 86_400;

    #[test]
    fn a_line_is_stale_when_it_has_been_idle_longer_than_its_lifetime() {
        let mut marks = Marks::default();
        let now = 1_000 * DAY;
        marks.touch(Hist::Commands, "make", now - 10 * DAY);
        marks.touch(Hist::Commands, "cargo test", now - 200 * DAY);

        assert!(!marks.is_stale(Hist::Commands, "make", now, 90));
        assert!(marks.is_stale(Hist::Commands, "cargo test", now, 90));
        // Nought is for ever, and a line nobody has a mark for is nobody's to drop.
        assert!(!marks.is_stale(Hist::Commands, "cargo test", now, 0));
        assert!(!marks.is_stale(Hist::Commands, "who is this", now, 90));
        // Using it again makes it young.
        marks.touch(Hist::Commands, "cargo test", now);
        assert!(!marks.is_stale(Hist::Commands, "cargo test", now, 90));
    }

    #[test]
    fn a_pinned_line_never_goes_stale_and_is_remembered_as_pinned() {
        let mut marks = Marks::default();
        let now = 1_000 * DAY;
        marks.touch(Hist::Files, "/etc/hosts", now - 400 * DAY);
        assert!(marks.is_stale(Hist::Files, "/etc/hosts", now, 90));

        assert!(marks.flip(Hist::Files, "/etc/hosts", now));
        assert!(marks.is_locked(Hist::Files, "/etc/hosts"));
        assert!(!marks.is_stale(Hist::Files, "/etc/hosts", now, 90));
        assert_eq!(marks.locked_lines(Hist::Files), vec!["/etc/hosts"]);

        let back = Marks::from_state(&marks.to_state());
        assert!(back.is_locked(Hist::Files, "/etc/hosts"));
        assert!(!back.is_locked(Hist::Commands, "/etc/hosts"));

        // And flipping it back lets it go.
        assert!(!marks.flip(Hist::Files, "/etc/hosts", now));
        assert!(marks.is_stale(Hist::Files, "/etc/hosts", now, 90));
    }

    #[test]
    fn trimming_drops_the_oldest_lines_and_passes_over_the_pinned_ones() {
        let line = |s: &str| s.to_string();
        let mut oldest_first = vec![line("a"), line("b"), line("c"), line("d")];
        trim(&mut oldest_first, 2, &[], true);
        assert_eq!(oldest_first, ["c", "d"]);

        let mut pinned = vec![line("a"), line("b"), line("c"), line("d")];
        trim(&mut pinned, 2, &[line("a")], true);
        assert_eq!(
            pinned,
            ["a", "d"],
            "the pinned one stays, the next oldest goes"
        );

        // The other way round: the files read or edited are kept newest first.
        let mut newest_first = vec![line("d"), line("c"), line("b"), line("a")];
        trim(&mut newest_first, 2, &[line("a")], false);
        assert_eq!(newest_first, ["d", "a"]);

        // Nothing to do, and nothing done.
        let mut short = vec![line("a")];
        trim(&mut short, 5, &[], true);
        assert_eq!(short, ["a"]);
    }

    #[test]
    fn marks_do_not_outlive_the_lines_they_describe() {
        let mut marks = Marks::default();
        marks.touch(Hist::Folders, "/tmp", 1);
        marks.touch(Hist::Folders, "/srv", 1);
        marks.touch(Hist::Commands, "/tmp", 1);

        marks.keep_only(Hist::Folders, &["/srv".to_string()]);
        assert_eq!(marks.to_state().len(), 2, "one folder and the command");
        assert!(marks.to_state().iter().any(|r| r.kind == "commands"));
    }
}
