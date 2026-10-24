//! The walk: roots in, [`Event`]s out, nothing kept in between.

use std::collections::HashSet;
use std::time::SystemTime;

use scmd_core::{CancelToken, Entry, EntryKind, Error, Filter, VPath};
use scmd_view::Codepage;

use crate::content::{Content, ContentError, Scanned, Scanner};
use crate::fs::{FindFs, VisitId};

/// What to look for.
///
/// Every condition on the *name*, the size, the dates and the attributes is a
/// [`Filter`](scmd_core::Filter), so the find dialog, the panel's filter bar and the copy
/// engine all speak one grammar and there is only ever one mask matcher.
#[derive(Debug, Clone, Default)]
pub struct Query {
    /// Where to start. Several roots, because FAR searches the selected items as well as a
    /// single directory. A root listed twice is read once.
    pub roots: Vec<VPath>,
    pub filter: Filter,
    pub subdirectories: bool,
    /// Follow a symbolic link that points at a directory. The walk remembers where it has
    /// been either way, so a loop ends a branch instead of the search.
    pub follow_links: bool,
    /// FAR's "include folders": may a directory be a result at all? It is *and*-ed with the
    /// filter's own kind condition, so `include_dirs` without `KindFilter::Dirs` means
    /// "files and folders" and with it means "folders only".
    pub include_dirs: bool,
    /// What the file has to contain, or not contain. A directory has no bytes, so a content
    /// condition excludes every directory even when `include_dirs` is on.
    pub content: Option<Content>,
}

impl Query {
    /// A search of one directory and everything under it, with no conditions at all.
    pub fn under(root: VPath) -> Query {
        Query {
            roots: vec![root],
            subdirectories: true,
            ..Query::default()
        }
    }

    pub fn with_filter(mut self, filter: Filter) -> Query {
        self.filter = filter;
        self
    }

    pub fn containing(mut self, content: Content) -> Query {
        self.content = Some(content);
        self
    }

    /// Is the content condition usable? The dialog calls this while the user is still typing,
    /// so a bad hex pattern is a message under the field rather than a search that refuses
    /// to start.
    pub fn check(&self) -> Result<(), ContentError> {
        match &self.content {
            Some(content) => Scanner::compile(content).map(|_| ()),
            None => Ok(()),
        }
    }
}

/// One result. Not comparable, because `Entry` is not: two listings of one file are the same
/// hit even when a timestamp moved under them.
#[derive(Debug, Clone)]
pub struct Hit {
    pub path: VPath,
    /// The entry as the directory listing gave it, so the result list can show a size, a date
    /// and the attributes without a second `stat`.
    pub entry: Entry,
    /// Where the needle was found. `None` when there was no content condition, and when the
    /// condition was `not_containing` — an absence has no offset to point at.
    pub offset: Option<u64>,
    /// Which code page the needle was found in, for a search over several of them.
    pub codepage: Option<Codepage>,
}

/// Why a path is not in the results although it might have belonged there. Reported rather
/// than swallowed: "no hits" and "nothing could be read" must not look the same.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trouble {
    /// A directory that could not be read; everything under it is missing from the results.
    Directory,
    /// A file that could not be opened or read while the content condition was being tested.
    File,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub path: VPath,
    pub what: Trouble,
    pub message: String,
}

/// What the search tells the caller, as it happens.
#[derive(Debug, Clone)]
pub enum Event {
    Hit(Hit),
    /// The directory the walk has just started reading, for the status line.
    Scanning(VPath),
    Problem(Problem),
}

/// What the search did, so the dialog can say so when it is over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Stats {
    /// Non-directory entries examined.
    pub files_seen: u64,
    /// Directories examined: every root, and every directory entry found under them.
    pub dirs_seen: u64,
    /// Bytes pulled off the disk to test content conditions. Nothing else reads a file.
    pub bytes_read: u64,
    pub hits: u64,
    pub problems: u64,
    pub cancelled: bool,
}

/// Run a search, reporting each result through `on` the moment it is found.
///
/// Nothing is collected: the only state that grows with the tree is the list of directories
/// still to visit and the set of directories already visited, both of which the walk needs
/// to be a walk at all. A caller that wants a list builds one in its own callback, and a
/// caller that wants to show hits while the search runs simply does.
///
/// `cancel` is checked before each directory and before each entry inside it, and again
/// between the windows of a long file, so `Esc` is answered in milliseconds and not after
/// the current directory.
///
/// The only error is a content condition that will not compile; everything that goes wrong
/// on the filesystem is an [`Event::Problem`] and the search carries on.
pub fn find(
    fs: &dyn FindFs,
    query: &Query,
    cancel: &CancelToken,
    on: &mut dyn FnMut(Event),
) -> Result<Stats, ContentError> {
    let scanner = query.content.as_ref().map(Scanner::compile).transpose()?;
    // One instant for the whole search, so `mtime > 7d` cannot mean two different things in
    // two different directories.
    let now = SystemTime::now();

    let mut stats = Stats::default();
    let mut visited: HashSet<VisitId> = HashSet::new();
    // A stack, so the walk is depth-first and the status line moves down one branch at a
    // time instead of jumping about. The flag is "count this one": a root has no parent
    // listing to have been counted in, every other directory was counted as an entry of its
    // parent, and counting it again would report every directory twice.
    let mut pending: Vec<(VPath, bool)> = query
        .roots
        .iter()
        .rev()
        .map(|r| (r.clone(), true))
        .collect();

    while let Some((dir, count_it)) = pending.pop() {
        if cancel.is_cancelled() {
            stats.cancelled = true;
            return Ok(stats);
        }

        // Identity before listing: a link, a bind mount or a second name for a directory we
        // have already read ends the branch here.
        let id = fs
            .visit_id(&dir)
            .unwrap_or_else(|| VisitId::Path(dir.to_string()));
        if !visited.insert(id) {
            continue;
        }
        if count_it {
            stats.dirs_seen += 1;
        }
        on(Event::Scanning(dir.clone()));

        let entries = match fs.list(&dir, cancel) {
            Ok(entries) => entries,
            Err(Error::Cancelled) => {
                stats.cancelled = true;
                return Ok(stats);
            }
            Err(e) => {
                report_problem(&mut stats, on, dir, Trouble::Directory, e.to_string());
                continue;
            }
        };

        let mut children: Vec<VPath> = Vec::new();
        for entry in entries {
            // `..` is the same directory we came from, under another name.
            if entry.kind == EntryKind::Parent {
                continue;
            }
            if cancel.is_cancelled() {
                stats.cancelled = true;
                return Ok(stats);
            }

            let path = dir.join(&entry.name);
            // Whether a link points at a directory is worth a `stat` in two cases: the walk
            // may want to go through it, or a content condition is about to try to read its
            // bytes. `/usr/include` holds three such links, and the second case is what
            // keeps them from being reported as three files that could not be read.
            let links_to_dir = entry.kind == EntryKind::Symlink
                && (query.follow_links || scanner.is_some())
                && fs.is_dir_following(&path);
            let dirlike = entry.kind == EntryKind::Dir || (links_to_dir && query.follow_links);
            if dirlike {
                stats.dirs_seen += 1;
            } else {
                stats.files_seen += 1;
            }
            if dirlike && query.subdirectories {
                children.push(path.clone());
            }

            if !query.filter.matches_at(&entry, now) {
                continue;
            }
            if dirlike && !query.include_dirs {
                continue;
            }

            match &scanner {
                // A directory has no bytes, whether we arrived at it by its own name or
                // through a link: a content condition is a condition on a file.
                Some(_) if dirlike || links_to_dir => {}
                Some(scanner) => {
                    let mut reader = match fs.open(&path) {
                        Ok(reader) => reader,
                        Err(e) => {
                            report_problem(&mut stats, on, path, Trouble::File, e.to_string());
                            continue;
                        }
                    };
                    match scanner.scan(&mut *reader, cancel, &mut stats.bytes_read) {
                        Err(e) => {
                            report_problem(&mut stats, on, path, Trouble::File, e.to_string());
                        }
                        Ok(Scanned::Cancelled) => {
                            stats.cancelled = true;
                            return Ok(stats);
                        }
                        Ok(scanned) => {
                            if scanner.decides(&scanned) {
                                let (offset, codepage) = match scanned {
                                    Scanned::Found { offset, codepage } => (Some(offset), codepage),
                                    _ => (None, None),
                                };
                                stats.hits += 1;
                                on(Event::Hit(Hit {
                                    path,
                                    entry,
                                    offset,
                                    codepage,
                                }));
                            }
                        }
                    }
                }
                None => {
                    stats.hits += 1;
                    on(Event::Hit(Hit {
                        path,
                        entry,
                        offset: None,
                        codepage: None,
                    }));
                }
            }
        }

        // Reversed, so popping the stack hands the children back in listing order.
        pending.extend(children.into_iter().rev().map(|child| (child, false)));
    }

    Ok(stats)
}

fn report_problem(
    stats: &mut Stats,
    on: &mut dyn FnMut(Event),
    path: VPath,
    what: Trouble,
    message: String,
) {
    stats.problems += 1;
    on(Event::Problem(Problem {
        path,
        what,
        message,
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    use scmd_core::{KindFilter, MaskSet, SizeRange};
    use scmd_vfs::LocalVfs;

    use crate::content::Needle;

    /// A small tree with two levels, a dotfile and three extensions.
    fn tree() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("one.rs"), b"fn main() { println!(\"hello\"); }").unwrap();
        fs::write(root.join("two.txt"), b"hello world").unwrap();
        fs::write(root.join(".hidden"), b"secret").unwrap();
        fs::create_dir(root.join("sub")).unwrap();
        fs::write(root.join("sub").join("three.rs"), b"// nothing here").unwrap();
        fs::write(root.join("sub").join("four.txt"), b"HELLO AGAIN").unwrap();
        fs::create_dir(root.join("sub").join("deep")).unwrap();
        fs::write(root.join("sub").join("deep").join("five.rs"), b"deep").unwrap();
        dir
    }

    /// Run a search and collect what the callback was told, in order.
    fn run(query: &Query) -> (Vec<Event>, Stats) {
        let mut events = Vec::new();
        let stats = find(&LocalVfs, query, &CancelToken::new(), &mut |e| {
            events.push(e)
        })
        .unwrap();
        (events, stats)
    }

    fn names(events: &[Event]) -> Vec<String> {
        let mut out: Vec<String> = events
            .iter()
            .filter_map(|e| match e {
                Event::Hit(hit) => Some(hit.path.display_name()),
                _ => None,
            })
            .collect();
        out.sort();
        out
    }

    fn masked(root: &Path, masks: &str) -> Query {
        Query::under(VPath::local(root))
            .with_filter(Filter::from_masks(MaskSet::parse(masks).unwrap()))
    }

    #[test]
    fn a_search_with_no_conditions_finds_every_file_under_the_roots() {
        let dir = tree();
        let (events, stats) = run(&Query::under(VPath::local(dir.path())));
        assert_eq!(
            names(&events),
            [".hidden", "five.rs", "four.txt", "one.rs", "three.rs", "two.txt"]
        );
        assert_eq!(stats.files_seen, 6);
        // The root, `sub`, `deep`: each counted once.
        assert_eq!(stats.dirs_seen, 3);
        assert_eq!(stats.hits, 6);
        assert_eq!(stats.bytes_read, 0, "no content condition reads no file");
        assert!(!stats.cancelled);
        assert_eq!(stats.problems, 0);
    }

    #[test]
    fn the_masks_the_panel_filter_speaks_are_the_masks_the_search_speaks() {
        let dir = tree();
        let (events, _) = run(&masked(dir.path(), "*.rs"));
        assert_eq!(names(&events), ["five.rs", "one.rs", "three.rs"]);

        let (events, _) = run(&masked(dir.path(), "*.rs,*.txt|three.*"));
        assert_eq!(names(&events), ["five.rs", "four.txt", "one.rs", "two.txt"]);
    }

    #[test]
    fn the_whole_filter_grammar_applies_not_only_the_masks() {
        let dir = tree();
        let mut query = Query::under(VPath::local(dir.path()));
        // Sizes in the tree: 32, 15, 11, 11, 6 and 4 bytes.
        query.filter.size = Some(SizeRange {
            min: Some(16),
            max: None,
        });
        assert_eq!(names(&run(&query).0), ["one.rs"]);

        query.filter.size = Some(SizeRange {
            min: Some(5),
            max: Some(14),
        });
        assert_eq!(names(&run(&query).0), [".hidden", "four.txt", "two.txt"]);
    }

    #[test]
    fn without_subdirectories_only_the_roots_own_entries_are_looked_at() {
        let dir = tree();
        let mut query = Query::under(VPath::local(dir.path()));
        query.subdirectories = false;
        let (events, stats) = run(&query);
        assert_eq!(names(&events), [".hidden", "one.rs", "two.txt"]);
        assert_eq!(stats.files_seen, 3);
        // The root plus the `sub` entry it holds, which was looked at but not entered.
        assert_eq!(stats.dirs_seen, 2);
    }

    #[test]
    fn folders_are_results_only_when_they_are_asked_for() {
        let dir = tree();
        let mut query = Query::under(VPath::local(dir.path()));
        assert!(!names(&run(&query).0).contains(&"sub".to_string()));

        query.include_dirs = true;
        assert_eq!(
            names(&run(&query).0),
            [".hidden", "deep", "five.rs", "four.txt", "one.rs", "sub", "three.rs", "two.txt"]
        );

        // `include_dirs` with a kind condition of its own means folders and nothing else.
        query.filter.kind = KindFilter::Dirs;
        assert_eq!(names(&run(&query).0), ["deep", "sub"]);
    }

    #[test]
    fn several_roots_are_searched_and_a_root_given_twice_is_read_once() {
        let dir = tree();
        let query = Query {
            roots: vec![
                VPath::local(dir.path().join("sub")),
                VPath::local(dir.path().join("sub")),
            ],
            subdirectories: true,
            ..Query::default()
        };
        let (events, stats) = run(&query);
        assert_eq!(names(&events), ["five.rs", "four.txt", "three.rs"]);
        assert_eq!(stats.dirs_seen, 2, "sub and deep, each once");
    }

    #[test]
    fn a_content_condition_is_tested_on_the_bytes_and_reports_where_it_matched() {
        let dir = tree();
        let query = Query::under(VPath::local(dir.path()))
            .containing(Content::containing(Needle::text("hello")));
        let (events, stats) = run(&query);
        assert_eq!(names(&events), ["four.txt", "one.rs", "two.txt"]);
        assert!(stats.bytes_read > 0);

        let hit = events
            .iter()
            .find_map(|e| match e {
                Event::Hit(hit) if hit.path.display_name() == "two.txt" => Some(hit),
                _ => None,
            })
            .unwrap();
        assert_eq!(hit.offset, Some(0));
        assert_eq!(hit.codepage, Some(Codepage::Utf8));
        assert_eq!(hit.entry.size, 11, "the listing's own facts come along");
    }

    #[test]
    fn a_content_condition_leaves_folders_out_even_when_folders_are_asked_for() {
        let dir = tree();
        let mut query = Query::under(VPath::local(dir.path()))
            .containing(Content::containing(Needle::text("hello")));
        query.include_dirs = true;
        assert_eq!(names(&run(&query).0), ["four.txt", "one.rs", "two.txt"]);
    }

    #[test]
    fn not_containing_finds_the_files_the_needle_is_absent_from() {
        let dir = tree();
        let query = Query::under(VPath::local(dir.path()))
            .containing(Content::not_containing(Needle::text("hello")));
        let (events, _) = run(&query);
        assert_eq!(names(&events), [".hidden", "five.rs", "three.rs"]);
        for event in &events {
            if let Event::Hit(hit) = event {
                assert_eq!(hit.offset, None, "an absence has no offset");
            }
        }
    }

    #[test]
    fn a_hex_needle_is_searched_over_the_tree_like_any_other() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.bin"), [0x00, 0x4d, 0x5a, 0x90]).unwrap();
        fs::write(dir.path().join("b.bin"), [0x00, 0x4d, 0x5b, 0x90]).unwrap();
        let query = Query::under(VPath::local(dir.path()))
            .containing(Content::containing(Needle::hex("4d 5a").unwrap()));
        let (events, _) = run(&query);
        assert_eq!(names(&events), ["a.bin"]);
    }

    #[test]
    fn a_broken_content_condition_is_refused_before_the_first_directory_is_read() {
        let dir = tree();
        let query = Query::under(VPath::local(dir.path()))
            .containing(Content::containing(Needle::text("")));
        let mut events = Vec::new();
        let err = find(&LocalVfs, &query, &CancelToken::new(), &mut |e| {
            events.push(e)
        })
        .unwrap_err();
        assert_eq!(err, ContentError::Empty);
        assert!(events.is_empty(), "not even a status line");
        assert_eq!(query.check(), Err(ContentError::Empty));
    }

    #[test]
    fn the_directory_being_read_is_announced_before_its_entries_are() {
        let dir = tree();
        let (events, _) = run(&Query::under(VPath::local(dir.path())));
        assert!(matches!(events.first(), Some(Event::Scanning(_))));
        let scanned: Vec<String> = events
            .iter()
            .filter_map(|e| match e {
                Event::Scanning(path) => Some(path.display_name()),
                _ => None,
            })
            .collect();
        assert_eq!(scanned.len(), 3);
        assert_eq!(scanned[1..], ["sub", "deep"]);
    }

    #[test]
    fn results_are_streamed_so_a_caller_can_stop_after_the_first_one() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..200 {
            fs::write(dir.path().join(format!("f{i:03}.txt")), b"x").unwrap();
        }
        let cancel = CancelToken::new();
        let mut hits = 0u64;
        // The callback cancels the moment it has what it wants, which can only work if the
        // engine is handing results over as it finds them.
        let stats = find(
            &LocalVfs,
            &Query::under(VPath::local(dir.path())),
            &cancel,
            &mut |event| {
                if let Event::Hit(_) = event {
                    hits += 1;
                    cancel.cancel();
                }
            },
        )
        .unwrap();
        assert_eq!(hits, 1);
        assert!(stats.cancelled);
        assert!(stats.files_seen < 200, "{}", stats.files_seen);
    }

    #[test]
    fn cancellation_is_answered_between_files_not_only_between_directories() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..200 {
            fs::write(dir.path().join(format!("f{i:03}.txt")), b"x").unwrap();
        }
        let cancel = CancelToken::new();
        let mut seen = 0u64;
        let stats = find(
            &LocalVfs,
            &Query::under(VPath::local(dir.path())),
            &cancel,
            &mut |event| {
                if let Event::Hit(_) = event {
                    seen += 1;
                    if seen == 5 {
                        cancel.cancel();
                    }
                }
            },
        )
        .unwrap();
        assert!(stats.cancelled);
        // One directory: had the check been at the directory boundary only, all 200 entries
        // would have been examined.
        assert!(stats.files_seen <= 6, "{}", stats.files_seen);
    }

    #[test]
    fn a_directory_that_cannot_be_read_is_reported_and_the_search_goes_on() {
        let dir = tree();
        let query = Query {
            roots: vec![
                VPath::local(dir.path().join("nowhere")),
                VPath::local(dir.path().join("sub")),
            ],
            subdirectories: true,
            ..Query::default()
        };
        let (events, stats) = run(&query);
        assert_eq!(stats.problems, 1);
        assert_eq!(names(&events), ["five.rs", "four.txt", "three.rs"]);
        let problem = events
            .iter()
            .find_map(|e| match e {
                Event::Problem(p) => Some(p),
                _ => None,
            })
            .unwrap();
        assert_eq!(problem.what, Trouble::Directory);
        assert!(problem.message.contains("nowhere"), "{}", problem.message);
    }

    #[test]
    fn a_file_that_cannot_be_read_is_a_problem_rather_than_a_file_without_the_needle() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("readable.txt"), b"needle").unwrap();
        fs::write(dir.path().join("closed.txt"), b"needle").unwrap();
        // A file whose bytes we are not allowed to see must not be reported as a file that
        // does not contain the needle, which is what a silent skip would amount to.
        let query = Query::under(VPath::local(dir.path()))
            .containing(Content::not_containing(Needle::text("needle")));

        struct Deny(LocalVfs);
        impl FindFs for Deny {
            fn list(&self, dir: &VPath, cancel: &CancelToken) -> scmd_core::Result<Vec<Entry>> {
                self.0.list(dir, cancel)
            }
            fn open(&self, path: &VPath) -> std::io::Result<Box<dyn std::io::Read + Send>> {
                if path.display_name() == "closed.txt" {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "permission denied",
                    ));
                }
                self.0.open(path)
            }
        }

        let mut events = Vec::new();
        let stats = find(&Deny(LocalVfs), &query, &CancelToken::new(), &mut |e| {
            events.push(e)
        })
        .unwrap();
        assert_eq!(stats.problems, 1);
        assert_eq!(stats.hits, 0);
        let problem = events
            .iter()
            .find_map(|e| match e {
                Event::Problem(p) => Some(p),
                _ => None,
            })
            .unwrap();
        assert_eq!(problem.what, Trouble::File);
        assert_eq!(problem.path.display_name(), "closed.txt");
    }

    #[test]
    fn the_limit_is_what_a_search_over_a_tree_actually_reads() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..4 {
            fs::write(dir.path().join(format!("f{i}.bin")), vec![b'.'; 100_000]).unwrap();
        }
        let query = Query::under(VPath::local(dir.path()))
            .containing(Content::not_containing(Needle::text("absent")).within(1_000));
        let (_, stats) = run(&query);
        assert_eq!(stats.hits, 4);
        assert_eq!(stats.bytes_read, 4_000);
    }

    #[cfg(unix)]
    #[test]
    fn a_link_to_a_directory_is_not_a_file_that_could_not_be_read() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir(root.join("real")).unwrap();
        fs::write(root.join("real").join("inside.txt"), b"needle").unwrap();
        std::os::unix::fs::symlink(root.join("real"), root.join("alias")).unwrap();
        std::os::unix::fs::symlink(root.join("gone"), root.join("dangling")).unwrap();

        // Links are not followed, so `alias` is an entry in its own right — but it still has
        // no bytes, and a content condition must not make a problem of that. A link to
        // nowhere has no bytes either, and that one is a problem: it is unreadable.
        let query = Query::under(VPath::local(root))
            .containing(Content::containing(Needle::text("needle")));
        let (events, stats) = run(&query);
        assert_eq!(names(&events), ["inside.txt"]);
        assert_eq!(stats.problems, 1);
        let problem = events
            .iter()
            .find_map(|e| match e {
                Event::Problem(p) => Some(p),
                _ => None,
            })
            .unwrap();
        assert_eq!(problem.path.display_name(), "dangling");
    }

    #[cfg(unix)]
    #[test]
    fn a_symbolic_link_loop_does_not_hang_the_walk() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("a").join("b")).unwrap();
        fs::write(root.join("a").join("b").join("leaf.txt"), b"x").unwrap();
        // `back` points at its own grandparent: following it for ever is the failure mode.
        std::os::unix::fs::symlink(root.join("a"), root.join("a").join("b").join("back")).unwrap();

        let mut query = Query::under(VPath::local(root));
        query.follow_links = true;
        let (events, stats) = run(&query);
        assert_eq!(names(&events), ["leaf.txt"]);
        // The root, `a`, `a/b`, and `a/b/back` — which is `a` again, so it is not re-read.
        assert_eq!(stats.dirs_seen, 4);
    }

    #[cfg(unix)]
    #[test]
    fn a_link_to_a_directory_is_only_entered_when_links_are_followed() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir(root.join("real")).unwrap();
        fs::write(root.join("real").join("inside.txt"), b"x").unwrap();
        std::os::unix::fs::symlink(root.join("real"), root.join("alias")).unwrap();

        let mut query = Query::under(VPath::local(root));
        // Not followed: the link is one entry, and `real` is read once.
        let (events, stats) = run(&query);
        assert_eq!(names(&events), ["alias", "inside.txt"]);
        assert_eq!(stats.files_seen, 2, "the link counts as a file of its own");

        // Followed: `alias` and `real` are the same directory, so its file is reported once.
        query.follow_links = true;
        let (events, _) = run(&query);
        assert_eq!(names(&events), ["inside.txt"]);
    }

    #[test]
    fn a_real_tree_is_walked_whole() {
        // The repository's own crates, as a last check that the walk is a walk: hundreds of
        // entries, real directory depth, and a content condition on every one of them.
        let root = VPath::local(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let query = Query::under(root.clone())
            .with_filter(Filter::from_masks(MaskSet::parse("*.rs").unwrap()))
            .containing(Content::containing(Needle::text("CancelToken")));
        let (events, stats) = run(&query);
        assert!(stats.files_seen > 100, "{}", stats.files_seen);
        assert!(stats.dirs_seen > 5, "{}", stats.dirs_seen);
        assert!(
            names(&events).contains(&"cancel.rs".to_string()),
            "{:?}",
            names(&events)
        );
        assert_eq!(stats.hits, names(&events).len() as u64);
    }
}
