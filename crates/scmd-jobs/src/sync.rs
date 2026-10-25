//! Making two directories hold the same thing.
//!
//! A synchronisation is not an operation — it is a *decision* about every file in two trees,
//! and then operations. So this module does only the deciding: it walks both sides, pairs them
//! up by the path they have in common, and says what would have to happen to each pair. What
//! happens next is the copy and delete the engine already has, which is why there is no
//! `JobKind::Sync`: the review table hands out ordinary jobs, and every answer the user gave
//! in it is a row in a list of paths.
//!
//! That split is also what makes it safe to look at. Nothing here writes, so the table on
//! screen is the whole of what was decided, and the user can change any row of it before a
//! single byte moves.

use std::collections::BTreeMap;
use std::io;
use std::time::{Duration, SystemTime};

use scmd_core::{CancelToken, EntryKind, MaskSet, VPath};

use crate::fs::{Facts, JobFs};
use crate::sum::{self, Algorithm};

/// What would have to happen to one pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Copy the left one over the right.
    Right,
    /// Copy the right one over the left.
    Left,
    /// Leave both alone.
    Skip,
    /// Take the right one away.
    DeleteRight,
    /// Take the left one away.
    DeleteLeft,
}

impl Action {
    /// How the table draws it: an arrow for a copy, a sign for the rest.
    pub fn mark(self) -> &'static str {
        match self {
            Action::Right => "\u{2192}",
            Action::Left => "\u{2190}",
            Action::Skip => "=",
            Action::DeleteRight => "\u{2192}\u{2612}",
            Action::DeleteLeft => "\u{2612}\u{2190}",
        }
    }

    /// The next answer the key offers for this pair, given which sides exist.
    ///
    /// The ring only ever offers what the pair allows: there is nothing to copy from a side
    /// that does not have the file, and nothing to delete from one that does not either.
    pub fn next(self, has_left: bool, has_right: bool) -> Action {
        let ring: Vec<Action> = [
            Action::Skip,
            Action::Right,
            Action::Left,
            Action::DeleteRight,
            Action::DeleteLeft,
        ]
        .into_iter()
        .filter(|a| match a {
            Action::Right => has_left,
            Action::Left => has_right,
            Action::DeleteRight => has_right,
            Action::DeleteLeft => has_left,
            Action::Skip => true,
        })
        .collect();
        let at = ring.iter().position(|a| *a == self).unwrap_or(0);
        ring[(at + 1) % ring.len()]
    }
}

/// What the two sides are, which is what the table colours a row by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Only the left side has it.
    OnlyLeft,
    /// Only the right side has it.
    OnlyRight,
    /// Both have it and they are the same.
    Same,
    /// Both have it and they differ; the newer side is the one the default action copies.
    Differ,
    /// Both have it, they differ, and nothing says which should win — the same time and a
    /// different size, or a file on one side and a directory on the other.
    Conflict,
}

/// One path, as both sides have it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pair {
    /// Where it is, relative to the two roots, with `/` between the parts.
    pub rel: String,
    pub left: Option<Facts>,
    pub right: Option<Facts>,
    pub verdict: Verdict,
    /// What will happen, which is a default until somebody changes it.
    pub action: Action,
}

impl Pair {
    pub fn is_dir(&self) -> bool {
        self.left
            .as_ref()
            .or(self.right.as_ref())
            .is_some_and(Facts::is_dir)
    }

    /// The bytes this row would move, for the line under the table.
    pub fn bytes(&self) -> u64 {
        match self.action {
            Action::Right => self.left.as_ref().map(|f| f.size).unwrap_or(0),
            Action::Left => self.right.as_ref().map(|f| f.size).unwrap_or(0),
            _ => 0,
        }
    }
}

/// How to decide, and what to offer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options {
    /// Walk into the directories both sides have. Off, this is the two directories themselves
    /// and nothing under them — which is what the compare command does.
    pub recursive: bool,
    /// Copy both ways, newer wins. Off, the left side is the source and the right one is made
    /// to match it.
    pub both_ways: bool,
    /// Offer to take out what the other side does not have. Off, an extra file is left alone.
    pub delete_extra: bool,
    /// Compare the bytes rather than the size and the time.
    pub by_content: bool,
    /// Only the names that match are paired up at all.
    pub filter: Option<MaskSet>,
}

/// Two file times count as the same when they are this close.
///
/// FAT stores a time to the nearest two seconds and so does a zip's own directory, so a file
/// copied to a memory stick and back comes home one second different through no fault of
/// anybody's. Treating that as a change would make every sync to a stick copy everything,
/// every time — which is the one thing a sync must not do.
pub const TIME_SLACK: Duration = Duration::from_secs(2);

/// Walk both sides and decide about every pair.
///
/// `progress` is called with each relative path as it is examined, so a sync of a large tree
/// says where it is rather than appearing to hang; returning from it does not stop the walk —
/// `cancel` does, and then the list is what was decided so far.
pub fn plan(
    fs: &dyn JobFs,
    left_root: &VPath,
    right_root: &VPath,
    options: &Options,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(&str, usize),
) -> io::Result<Vec<Pair>> {
    let mut pairs = Vec::new();
    walk(
        fs, left_root, right_root, "", options, cancel, progress, &mut pairs,
    )?;
    Ok(pairs)
}

#[allow(clippy::too_many_arguments)]
fn walk(
    fs: &dyn JobFs,
    left_root: &VPath,
    right_root: &VPath,
    prefix: &str,
    options: &Options,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(&str, usize),
    out: &mut Vec<Pair>,
) -> io::Result<()> {
    let here_left = at(left_root, prefix);
    let here_right = at(right_root, prefix);
    // One side missing is not an error: the caller is told about the directory itself, and
    // there is nothing to pair up inside it.
    let left = fs.read_dir(&here_left).unwrap_or_default();
    let right = fs.read_dir(&here_right).unwrap_or_default();

    let mut both: BTreeMap<String, (Option<Facts>, Option<Facts>)> = BTreeMap::new();
    for (name, facts) in left {
        both.entry(name.to_string_lossy().into_owned())
            .or_default()
            .0 = Some(facts);
    }
    for (name, facts) in right {
        both.entry(name.to_string_lossy().into_owned())
            .or_default()
            .1 = Some(facts);
    }

    for (name, (left, right)) in both {
        if cancel.is_cancelled() {
            return Ok(());
        }
        let rel = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        let dir = left.as_ref().or(right.as_ref()).is_some_and(Facts::is_dir);
        // A mask applies to files and never to directories, exactly as a copy's does: `*.rs`
        // means the Rust files in the tree, not the directories whose names end in `.rs`.
        if !dir {
            if let Some(filter) = &options.filter {
                if !filter.matches_name(&name) {
                    continue;
                }
            }
        }
        progress(&rel, out.len());

        // Two directories are walked rather than paired: what differs is inside them, and a
        // row saying "these two directories differ" is a row nobody can act on.
        if dir && left.is_some() && right.is_some() {
            if options.recursive {
                walk(
                    fs, left_root, right_root, &rel, options, cancel, progress, out,
                )?;
            }
            continue;
        }

        let (verdict, action) = decide(
            fs,
            &at(left_root, &rel),
            &at(right_root, &rel),
            left.as_ref(),
            right.as_ref(),
            options,
            cancel,
        );
        out.push(Pair {
            rel,
            left,
            right,
            verdict,
            action,
        });
    }
    Ok(())
}

/// What this pair is, and what should happen to it by default.
#[allow(clippy::too_many_arguments)]
fn decide(
    fs: &dyn JobFs,
    left_path: &VPath,
    right_path: &VPath,
    left: Option<&Facts>,
    right: Option<&Facts>,
    options: &Options,
    cancel: &CancelToken,
) -> (Verdict, Action) {
    match (left, right) {
        (Some(_), None) => (Verdict::OnlyLeft, Action::Right),
        (None, Some(_)) => (
            Verdict::OnlyRight,
            if options.both_ways {
                Action::Left
            } else if options.delete_extra {
                Action::DeleteRight
            } else {
                Action::Skip
            },
        ),
        (Some(l), Some(r)) => {
            // A file on one side and a directory on the other is not a difference to resolve
            // by copying: one of them has to go first, and that is the user's call.
            if l.is_dir() != r.is_dir() {
                return (Verdict::Conflict, Action::Skip);
            }
            if same(fs, left_path, right_path, l, r, options, cancel) {
                return (Verdict::Same, Action::Skip);
            }
            if !options.both_ways {
                return (Verdict::Differ, Action::Right);
            }
            match newer(l.mtime, r.mtime) {
                Some(true) => (Verdict::Differ, Action::Right),
                Some(false) => (Verdict::Differ, Action::Left),
                // The same time and different bytes: there is no rule that picks a side.
                None => (Verdict::Conflict, Action::Skip),
            }
        }
        (None, None) => (Verdict::Same, Action::Skip),
    }
}

/// Are the two the same file, as far as this sync is asked to care?
fn same(
    fs: &dyn JobFs,
    left_path: &VPath,
    right_path: &VPath,
    left: &Facts,
    right: &Facts,
    options: &Options,
    cancel: &CancelToken,
) -> bool {
    if left.kind == EntryKind::Dir && right.kind == EntryKind::Dir {
        return true;
    }
    if left.size != right.size {
        return false;
    }
    if !options.by_content {
        return close_enough(left.mtime, right.mtime);
    }
    // The bytes, by hashing both and comparing the digests. Two streams cannot be read side
    // by side through a filesystem that hands bytes to a callback rather than letting one
    // pull them, so this reads each once instead of both at once — twice the reads of an
    // interleaved comparison, and the only shape that works for every provider.
    let mut tick = |_: u64| !cancel.is_cancelled();
    let digest = |path: &VPath, tick: &mut dyn FnMut(u64) -> bool| {
        sum::digest(fs, path, Algorithm::Sha256, options_buffer(), tick).ok()
    };
    match (digest(left_path, &mut tick), digest(right_path, &mut tick)) {
        (Some(a), Some(b)) => a == b,
        // One of them could not be read: saying they are the same would quietly skip it.
        _ => false,
    }
}

fn options_buffer() -> usize {
    256 * 1024
}

/// Is the left one newer? `None` when there is nothing to tell them apart by.
fn newer(left: Option<SystemTime>, right: Option<SystemTime>) -> Option<bool> {
    let (l, r) = (left?, right?);
    if close_enough(Some(l), Some(r)) {
        return None;
    }
    Some(l > r)
}

fn close_enough(left: Option<SystemTime>, right: Option<SystemTime>) -> bool {
    match (left, right) {
        (Some(l), Some(r)) => {
            let gap = l.duration_since(r).or_else(|_| r.duration_since(l));
            gap.map(|d| d <= TIME_SLACK).unwrap_or(false)
        }
        // A provider that does not report times cannot be asked this question; the size is
        // all there is, and it matched.
        _ => true,
    }
}

/// `root` plus a relative path, one part at a time: a single `join` of `a/b` would be one
/// name containing a separator for a provider that builds its own paths.
pub fn at(root: &VPath, rel: &str) -> VPath {
    let mut out = root.clone();
    for part in rel.split('/').filter(|p| !p.is_empty()) {
        out = out.join(part);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::LocalFs;

    /// Two directories on disk, and a way to put a file in either of them.
    struct Two {
        _dir: tempfile::TempDir,
        left: VPath,
        right: VPath,
    }

    impl Two {
        fn new() -> Two {
            let dir = tempfile::tempdir().unwrap();
            std::fs::create_dir_all(dir.path().join("a")).unwrap();
            std::fs::create_dir_all(dir.path().join("b")).unwrap();
            Two {
                left: VPath::local(dir.path().join("a")),
                right: VPath::local(dir.path().join("b")),
                _dir: dir,
            }
        }

        /// Put `bytes` at `rel` on one side, optionally that much older than now.
        fn put(&self, side: &VPath, rel: &str, bytes: &[u8], age_secs: u64) {
            let path = side.as_path().join(rel);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(&path, bytes).unwrap();
            if age_secs > 0 {
                let when = SystemTime::now() - Duration::from_secs(age_secs);
                let file = std::fs::File::options().write(true).open(&path).unwrap();
                file.set_modified(when).unwrap();
            }
        }

        fn mkdir(&self, side: &VPath, rel: &str) {
            std::fs::create_dir_all(side.as_path().join(rel)).unwrap();
        }

        fn run(&self, options: &Options) -> Vec<Pair> {
            let mut nothing = |_: &str, _: usize| {};
            plan(
                &LocalFs,
                &self.left,
                &self.right,
                options,
                &CancelToken::new(),
                &mut nothing,
            )
            .expect("two readable directories")
        }
    }

    fn row<'a>(pairs: &'a [Pair], rel: &str) -> &'a Pair {
        let names: Vec<&str> = pairs.iter().map(|p| p.rel.as_str()).collect();
        pairs
            .iter()
            .find(|p| p.rel == rel)
            .unwrap_or_else(|| panic!("no row for `{rel}` in {names:?}"))
    }

    #[test]
    fn one_way_makes_the_right_side_match_the_left() {
        let t = Two::new();
        t.put(&t.left, "only-left.txt", b"x", 0);
        t.put(&t.right, "only-right.txt", b"y", 0);
        t.put(&t.left, "both.txt", b"new", 0);
        t.put(&t.right, "both.txt", b"old", 600);

        let pairs = t.run(&Options::default());
        assert_eq!(row(&pairs, "only-left.txt").action, Action::Right);
        assert_eq!(row(&pairs, "only-left.txt").verdict, Verdict::OnlyLeft);
        assert_eq!(row(&pairs, "both.txt").action, Action::Right);
        assert_eq!(row(&pairs, "both.txt").verdict, Verdict::Differ);
        // Nothing is taken away unless that was asked for.
        assert_eq!(row(&pairs, "only-right.txt").action, Action::Skip);
        assert_eq!(row(&pairs, "only-right.txt").verdict, Verdict::OnlyRight);
    }

    #[test]
    fn one_way_offers_to_take_out_what_the_left_side_does_not_have() {
        let t = Two::new();
        t.put(&t.right, "extra.txt", b"y", 0);
        let options = Options {
            delete_extra: true,
            ..Options::default()
        };
        let pairs = t.run(&options);
        assert_eq!(row(&pairs, "extra.txt").action, Action::DeleteRight);
    }

    #[test]
    fn both_ways_lets_the_newer_side_win_each_file_on_its_own() {
        let t = Two::new();
        t.put(&t.left, "left-newer.txt", b"aaa", 0);
        t.put(&t.right, "left-newer.txt", b"bbb", 600);
        t.put(&t.left, "right-newer.txt", b"aaa", 600);
        t.put(&t.right, "right-newer.txt", b"bbb", 0);
        t.put(&t.right, "only-right.txt", b"y", 0);

        let options = Options {
            both_ways: true,
            ..Options::default()
        };
        let pairs = t.run(&options);
        assert_eq!(row(&pairs, "left-newer.txt").action, Action::Right);
        assert_eq!(row(&pairs, "right-newer.txt").action, Action::Left);
        assert_eq!(
            row(&pairs, "only-right.txt").action,
            Action::Left,
            "both ways means a file only on the right comes over too"
        );
    }

    #[test]
    fn the_same_time_and_a_different_size_is_a_conflict_rather_than_a_guess() {
        let t = Two::new();
        // The same modification time, told to the second, and different bytes.
        t.put(&t.left, "clash.txt", b"short", 600);
        t.put(&t.right, "clash.txt", b"a good deal longer", 600);
        let options = Options {
            both_ways: true,
            ..Options::default()
        };
        let pairs = t.run(&options);
        assert_eq!(row(&pairs, "clash.txt").verdict, Verdict::Conflict);
        assert_eq!(
            row(&pairs, "clash.txt").action,
            Action::Skip,
            "nothing moves until somebody says which side wins"
        );
    }

    #[test]
    fn a_file_against_a_directory_is_a_conflict_and_not_a_copy() {
        let t = Two::new();
        t.put(&t.left, "thing", b"a file here", 0);
        t.mkdir(&t.right, "thing");
        let pairs = t.run(&Options::default());
        assert_eq!(row(&pairs, "thing").verdict, Verdict::Conflict);
        assert_eq!(row(&pairs, "thing").action, Action::Skip);
    }

    #[test]
    fn two_seconds_of_difference_is_the_same_file() {
        let t = Two::new();
        // What a trip through a memory stick or a zip does to a timestamp.
        t.put(&t.left, "copied.txt", b"same bytes", 600);
        t.put(&t.right, "copied.txt", b"same bytes", 601);
        let pairs = t.run(&Options::default());
        assert_eq!(row(&pairs, "copied.txt").verdict, Verdict::Same);
        assert_eq!(row(&pairs, "copied.txt").action, Action::Skip);
    }

    #[test]
    fn by_content_sees_through_a_time_that_lies_in_either_direction() {
        let t = Two::new();
        // The same bytes, hours apart: by time they differ, by content they do not.
        t.put(&t.left, "same.txt", b"the very same bytes", 0);
        t.put(&t.right, "same.txt", b"the very same bytes", 6000);
        // Different bytes of the same length at the same time: only content tells them apart.
        t.put(&t.left, "sneaky.txt", b"aaaa", 600);
        t.put(&t.right, "sneaky.txt", b"bbbb", 600);

        let by_time = t.run(&Options {
            both_ways: true,
            ..Options::default()
        });
        assert_eq!(row(&by_time, "same.txt").verdict, Verdict::Differ);
        assert_eq!(row(&by_time, "sneaky.txt").verdict, Verdict::Same);

        let by_content = t.run(&Options {
            both_ways: true,
            by_content: true,
            ..Options::default()
        });
        assert_eq!(row(&by_content, "same.txt").verdict, Verdict::Same);
        assert_eq!(row(&by_content, "sneaky.txt").verdict, Verdict::Conflict);
    }

    #[test]
    fn a_directory_both_sides_have_is_walked_and_one_only_one_side_has_travels_whole() {
        let t = Two::new();
        t.put(&t.left, "shared/inside.txt", b"x", 0);
        t.mkdir(&t.right, "shared");
        t.put(&t.left, "fresh/deep/one.txt", b"x", 0);

        let flat = t.run(&Options::default());
        assert!(
            !flat.iter().any(|p| p.rel == "shared/inside.txt"),
            "without `recursive` it stops at the two directories"
        );
        assert!(flat.iter().any(|p| p.rel == "fresh"));

        let deep = t.run(&Options {
            recursive: true,
            ..Options::default()
        });
        assert_eq!(row(&deep, "shared/inside.txt").action, Action::Right);
        // The new directory is one row, not one row per file inside it: the copy takes its
        // tree with it, and a row per file would be a hundred answers to one question.
        assert_eq!(row(&deep, "fresh").action, Action::Right);
        assert!(deep.iter().all(|p| p.rel != "fresh/deep/one.txt"));
    }

    #[test]
    fn a_mask_pairs_up_only_what_matches_and_never_hides_a_directory() {
        let t = Two::new();
        t.put(&t.left, "code.rs", b"x", 0);
        t.put(&t.left, "notes.txt", b"x", 0);
        t.put(&t.left, "deep/code.rs", b"x", 0);
        t.mkdir(&t.right, "deep");

        let options = Options {
            recursive: true,
            filter: Some(MaskSet::parse("*.rs").unwrap()),
            ..Options::default()
        };
        let pairs = t.run(&options);
        assert!(pairs.iter().any(|p| p.rel == "code.rs"));
        assert!(pairs.iter().any(|p| p.rel == "deep/code.rs"));
        assert!(
            !pairs.iter().any(|p| p.rel == "notes.txt"),
            "a mask leaves out what does not match"
        );
    }

    #[test]
    fn the_ring_of_answers_only_offers_what_the_pair_allows() {
        // Only the left side has it: there is nothing to copy leftwards and nothing to delete
        // on the right.
        let a = Action::Skip.next(true, false);
        assert_eq!(a, Action::Right);
        assert_eq!(a.next(true, false), Action::DeleteLeft);
        assert_eq!(Action::DeleteLeft.next(true, false), Action::Skip);
        // Both sides have it: every answer is on the ring.
        assert_eq!(Action::Skip.next(true, true), Action::Right);
        assert_eq!(Action::Right.next(true, true), Action::Left);
        assert_eq!(Action::Left.next(true, true), Action::DeleteRight);
        assert_eq!(Action::DeleteRight.next(true, true), Action::DeleteLeft);
        assert_eq!(Action::DeleteLeft.next(true, true), Action::Skip);
    }

    #[test]
    fn a_cancelled_walk_keeps_what_it_decided_rather_than_failing() {
        let t = Two::new();
        t.put(&t.left, "one.txt", b"x", 0);
        let cancel = CancelToken::new();
        cancel.cancel();
        let mut nothing = |_: &str, _: usize| {};
        let pairs = plan(
            &LocalFs,
            &t.left,
            &t.right,
            &Options::default(),
            &cancel,
            &mut nothing,
        )
        .expect("a cancelled walk is not an error");
        assert!(pairs.is_empty());
    }

    #[test]
    fn what_a_row_would_move_is_the_size_of_the_side_it_comes_from() {
        let t = Two::new();
        t.put(&t.left, "one.txt", b"12345", 0);
        let pairs = t.run(&Options::default());
        let pair = row(&pairs, "one.txt");
        assert_eq!(pair.bytes(), 5);
        let mut skipped = pair.clone();
        skipped.action = Action::Skip;
        assert_eq!(skipped.bytes(), 0, "a row that does nothing moves nothing");
    }
}
