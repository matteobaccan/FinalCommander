//! Asking an index instead of walking (SPEC 06 §1e).
//!
//! Every system these days keeps a list of its files somewhere: `plocate` or `locate` on
//! Linux, Spotlight on macOS, Everything on Windows. Asking one is milliseconds where a walk
//! of the same disk is minutes, and each of them comes with a small program that answers on
//! its standard output — which is the whole of how this module talks to them. Nothing is
//! linked, nothing is read out of anybody's database: a program on the `PATH` is run, and
//! what it prints is a list of candidates.
//!
//! **Candidates, never results.** An index is as old as its last update and knows a path and
//! little else, so everything it proposes goes through [`find_among`], which looks at each
//! file again on the disk and asks it everything the query asks. What the index is given is
//! therefore only a *hint* — where to look, and a few letters the name must hold when the
//! mask has any — and getting too much back is always safe.
//!
//! **Optional by construction.** No program, a program that will not start, one that says
//! its database is missing: each is [`IndexError::Unavailable`], and the caller walks
//! instead. The one thing that cannot be made up for is what the index has not seen yet — a
//! file made since its last update is not proposed, so it is not found. The dialog says so.

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use fcmd_core::{CancelToken, VPath};

use crate::content::ContentError;
use crate::fs::FindFs;
use crate::walk::{find_among, Event, Query, Stats};

/// Which program answers, because each takes its question differently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// `plocate`, and `locate` from mlocate or findutils: patterns matched against the
    /// whole path.
    Locate,
    /// macOS: Spotlight's `mdfind`.
    Spotlight,
    /// Windows: `es`, the command line of voidtools' Everything.
    Everything,
    /// A line from the configuration, with `{root}` and `{name}` filled in.
    Custom,
}

/// An index that can be asked: the program, and how to put the question to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Index {
    /// What to call it on the screen: `plocate`, `mdfind`, `es`.
    pub name: String,
    program: PathBuf,
    kind: Kind,
    /// The arguments of a configured command, placeholders and all.
    template: Vec<String>,
}

/// Why an indexed search did not happen.
#[derive(Debug)]
pub enum IndexError {
    /// The content condition will not compile — the same refusal the walk gives.
    Content(ContentError),
    /// The index could not be asked. The search is still possible: by walking.
    Unavailable(String),
}

impl std::fmt::Display for IndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexError::Content(e) => e.fmt(f),
            IndexError::Unavailable(why) => f.write_str(why),
        }
    }
}

impl Index {
    /// The index this machine has, if it has one: the first of the system's own programs
    /// that is on the `PATH`. Finding it is the shell's rule and nothing cleverer
    /// (`fcmd_arc::which`), the same as for every other program this project runs.
    pub fn detect() -> Option<Index> {
        Index::programs()
            .iter()
            .find_map(|(program, kind)| Index::found(program, *kind))
    }

    /// The programs looked for on this system, in order of preference: what the list of
    /// programs from elsewhere shows, found or not.
    pub fn program_names() -> Vec<&'static str> {
        Index::programs().iter().map(|(name, _)| *name).collect()
    }

    fn programs() -> &'static [(&'static str, Kind)] {
        if cfg!(windows) {
            &[("es", Kind::Everything)]
        } else if cfg!(target_os = "macos") {
            &[("mdfind", Kind::Spotlight)]
        } else {
            &[("plocate", Kind::Locate), ("locate", Kind::Locate)]
        }
    }

    fn found(program: &str, kind: Kind) -> Option<Index> {
        Some(Index {
            name: program.to_string(),
            program: fcmd_arc::which(program)?,
            kind,
            template: Vec::new(),
        })
    }

    /// An index somebody configured: a command line whose output is one path per line (or
    /// NUL-separated), with `{root}` replaced by the folder to look under and `{name}` by
    /// the letters the name must hold, which may be none. `None` when the line is empty or
    /// its program is not there.
    ///
    /// The line is split at spaces *before* the placeholders are filled in, so a folder
    /// with a space in its name is still one argument.
    pub fn custom(command: &str) -> Option<Index> {
        let mut words = command.split_whitespace();
        let program = words.next()?;
        Some(Index {
            name: Path::new(program)
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_else(|| program.to_string()),
            program: fcmd_arc::which(program)?,
            kind: Kind::Custom,
            template: words.map(str::to_string).collect(),
        })
    }

    /// The arguments that ask about one folder.
    fn arguments(&self, root: &Path, hint: &str) -> Vec<String> {
        let root = root.to_string_lossy().into_owned();
        match self.kind {
            Kind::Locate => {
                // `-0` for names with anything in them, `-i` because a mask ignores case
                // unless told otherwise and too many candidates is safe, `-A` so that
                // both patterns have to hold. A pattern with no wildcard in it is a
                // substring of the whole path — which is how the folder is said; a
                // folder whose own name has a wildcard in it cannot be said that way and
                // is left to the check afterwards, as the root of the disk is.
                let mut args = vec!["-0".to_string(), "-i".to_string(), "-A".to_string()];
                args.push("--".to_string());
                let sayable = !root.contains(['*', '?', '[', '\\']);
                let whole_disk = root == "/";
                if sayable && !(whole_disk && !hint.is_empty()) {
                    args.push(root);
                }
                if !hint.is_empty() {
                    args.push(hint.to_string());
                } else if !sayable {
                    args.push("/".to_string());
                }
                args
            }
            Kind::Spotlight => {
                let mut args = vec!["-0".to_string(), "-onlyin".to_string(), root];
                if hint.is_empty() {
                    args.push("kMDItemFSName == '*'".to_string());
                } else {
                    args.push("-name".to_string());
                    args.push(hint.to_string());
                }
                args
            }
            Kind::Everything => {
                let mut args = vec!["-path".to_string(), root];
                if !hint.is_empty() {
                    args.push(hint.to_string());
                }
                args
            }
            Kind::Custom => self
                .template
                .iter()
                .map(|word| word.replace("{root}", &root).replace("{name}", hint))
                .filter(|word| !word.is_empty())
                .collect(),
        }
    }
}

/// The letters a name must hold for a mask to match it, where a mask says any: `*.rs` gives
/// `.rs`, `report*2024*` gives `report`. Empty when the mask is more than one plain pattern
/// — a list, an exclusion, a `<group>`, a `/regex/`, a `[range]`, a mask that looks at the
/// path — because then no single run of letters is in every match; and empty when the run
/// is one letter, which narrows nothing.
pub fn hint(mask: &str) -> String {
    let mask = mask.trim();
    if mask.contains([',', ';', '|', '<', '>', '/', '\\', '[', ']', '{', '}', '"']) {
        return String::new();
    }
    let longest = mask
        .split(['*', '?'])
        .max_by_key(|run| run.chars().count())
        .unwrap_or("");
    if longest.chars().count() < 2 {
        return String::new();
    }
    longest.to_string()
}

/// Run a search whose candidates come from an index.
///
/// Each root of the query is asked about in turn, and what comes back is streamed straight
/// into [`find_among`] — nothing waits for the program to finish, so hits appear as the
/// index answers and `Esc` stops the program as well as the search.
///
/// Only folders on this machine's own filesystem can be asked about: a root on a server or
/// inside an archive is [`IndexError::Unavailable`], and so is a query that steps into
/// archives, which no index has looked inside.
pub fn find_indexed(
    fs: &dyn FindFs,
    query: &Query,
    index: &Index,
    hint: &str,
    cancel: &CancelToken,
    on: &mut dyn FnMut(Event),
) -> Result<Stats, IndexError> {
    query.check().map_err(IndexError::Content)?;
    if query.in_archives {
        return Err(IndexError::Unavailable(
            "an index does not look inside archives".to_string(),
        ));
    }
    if let Some(root) = query
        .roots
        .iter()
        .find(|root| !root.is_local() || root.in_container())
    {
        return Err(IndexError::Unavailable(format!(
            "{root} is not a folder {} knows",
            index.name
        )));
    }

    let mut total = Stats::default();
    for root in &query.roots {
        on(Event::Scanning(root.clone()));
        let mut child = Command::new(&index.program)
            .args(index.arguments(root.as_path(), hint))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| IndexError::Unavailable(format!("{}: {e}", index.name)))?;
        let (Some(out), Some(mut err)) = (child.stdout.take(), child.stderr.take()) else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(IndexError::Unavailable(format!(
                "{}: no output to read",
                index.name
            )));
        };
        // What it complains about is read beside what it answers, so a program that fills
        // one pipe while the other is being read never stops either side.
        let complaints = std::thread::spawn(move || {
            let mut text = String::new();
            let _ = (&mut err).take(64 * 1024).read_to_string(&mut text);
            text
        });

        let mut proposed = 0u64;
        let mut paths = Paths::new(BufReader::new(out)).inspect(|_| proposed += 1);
        // One root at a time, so a candidate is only ever kept for the root that was asked
        // about: a second root gets its own answer.
        let one_root = Query {
            roots: vec![root.clone()],
            ..query.clone()
        };
        let stats = find_among(fs, &one_root, &mut paths, cancel, on);
        drop(paths);
        if cancel.is_cancelled() {
            let _ = child.kill();
        }
        let status = child.wait();
        let said = complaints.join().unwrap_or_default();
        let stats = stats.map_err(IndexError::Content)?;

        // `locate` ends with 1 when it found nothing, which is an answer. A program that
        // ended badly, proposed nothing *and* had something to say for itself did not
        // answer at all: its database is missing, its service is not running.
        let failed = !status.is_ok_and(|s| s.success());
        if failed && proposed == 0 && !stats.cancelled {
            if let Some(why) = said.lines().map(str::trim).find(|line| !line.is_empty()) {
                return Err(IndexError::Unavailable(why.to_string()));
            }
        }

        total.files_seen += stats.files_seen;
        total.dirs_seen += stats.dirs_seen;
        total.bytes_read += stats.bytes_read;
        total.hits += stats.hits;
        total.problems += stats.problems;
        if stats.cancelled {
            total.cancelled = true;
            break;
        }
    }
    Ok(total)
}

/// The paths a program prints, one at a time as it prints them: separated by NUL or by a
/// line end, whichever it uses, so one reader serves `locate -0` and a script that echoes.
struct Paths<R> {
    from: R,
    done: bool,
}

impl<R: BufRead> Paths<R> {
    fn new(from: R) -> Paths<R> {
        Paths { from, done: false }
    }
}

impl<R: BufRead> Iterator for Paths<R> {
    type Item = VPath;

    fn next(&mut self) -> Option<VPath> {
        while !self.done {
            let mut line: Vec<u8> = Vec::new();
            loop {
                let (used, ended) = match self.from.fill_buf() {
                    Ok([]) | Err(_) => {
                        self.done = true;
                        break;
                    }
                    Ok(buffer) => match buffer.iter().position(|&b| b == 0 || b == b'\n') {
                        Some(at) => {
                            line.extend_from_slice(&buffer[..at]);
                            (at + 1, true)
                        }
                        None => {
                            line.extend_from_slice(buffer);
                            (buffer.len(), false)
                        }
                    },
                };
                self.from.consume(used);
                if ended {
                    break;
                }
            }
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            if !line.is_empty() {
                return Some(VPath::local(String::from_utf8_lossy(&line).into_owned()));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fcmd_core::{Filter, MaskSet};
    use fcmd_vfs::LocalVfs;

    fn index(kind: Kind) -> Index {
        Index {
            name: "test".into(),
            program: PathBuf::from("test"),
            kind,
            template: vec!["--under".into(), "{root}".into(), "{name}".into()],
        }
    }

    #[test]
    fn a_mask_gives_the_letters_a_name_must_hold_or_none() {
        assert_eq!(hint("*.rs"), ".rs");
        assert_eq!(hint("report*2024*"), "report");
        assert_eq!(hint("Cargo.toml"), "Cargo.toml");
        assert_eq!(hint("a?bcd*"), "bcd");
        // Nothing every match has in common, or nothing worth saying.
        for mask in [
            "*",
            "*.rs,*.toml",
            "*.rs|test*",
            "<arc>",
            "/^a/",
            "src/*.rs",
            "[a-c]*",
            "*a*",
        ] {
            assert_eq!(hint(mask), "", "{mask}");
        }
    }

    #[test]
    fn each_program_is_asked_in_its_own_words() {
        let root = Path::new("/home/mb/src");
        assert_eq!(
            index(Kind::Locate).arguments(root, ".rs"),
            ["-0", "-i", "-A", "--", "/home/mb/src", ".rs"]
        );
        assert_eq!(
            index(Kind::Locate).arguments(root, ""),
            ["-0", "-i", "-A", "--", "/home/mb/src"]
        );
        // The whole disk is in every path: with letters to look for it is left out, and
        // without them it is the only thing there is to say.
        assert_eq!(
            index(Kind::Locate).arguments(Path::new("/"), ".rs"),
            ["-0", "-i", "-A", "--", ".rs"]
        );
        assert_eq!(
            index(Kind::Locate).arguments(Path::new("/"), ""),
            ["-0", "-i", "-A", "--", "/"]
        );
        // A folder `locate` would read as a wildcard is checked afterwards instead.
        assert_eq!(
            index(Kind::Locate).arguments(Path::new("/data/[old]"), ".rs"),
            ["-0", "-i", "-A", "--", ".rs"]
        );
        assert_eq!(
            index(Kind::Locate).arguments(Path::new("/data/[old]"), ""),
            ["-0", "-i", "-A", "--", "/"]
        );
        assert_eq!(
            index(Kind::Spotlight).arguments(root, "report"),
            ["-0", "-onlyin", "/home/mb/src", "-name", "report"]
        );
        assert_eq!(
            index(Kind::Spotlight).arguments(root, ""),
            ["-0", "-onlyin", "/home/mb/src", "kMDItemFSName == '*'"]
        );
        assert_eq!(
            index(Kind::Everything).arguments(root, "report"),
            ["-path", "/home/mb/src", "report"]
        );
        // A configured line: the placeholders filled in, and an empty one dropped.
        assert_eq!(
            index(Kind::Custom).arguments(Path::new("/my files"), "x"),
            ["--under", "/my files", "x"]
        );
        assert_eq!(
            index(Kind::Custom).arguments(root, ""),
            ["--under", "/home/mb/src"]
        );
    }

    #[test]
    fn the_answer_is_read_whether_it_ends_its_paths_with_nul_or_with_a_line() {
        let read = |bytes: &[u8]| -> Vec<String> {
            Paths::new(BufReader::with_capacity(4, bytes))
                .map(|p| p.as_path().to_string_lossy().into_owned())
                .collect()
        };
        assert_eq!(read(b"/a/one\0/a/two\0"), ["/a/one", "/a/two"]);
        assert_eq!(read(b"/a/one\n/a/two"), ["/a/one", "/a/two"]);
        // Windows line ends, blank lines, and a name with a space in it.
        assert_eq!(
            read(b"C:\\a one\r\n\r\nC:\\two\r\n"),
            ["C:\\a one", "C:\\two"]
        );
        assert!(read(b"").is_empty());
    }

    /// The same tree the walk's own tests use, so the two can be compared.
    fn tree() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("one.rs"), b"fn main() {}").unwrap();
        std::fs::write(root.join("two.txt"), b"hello").unwrap();
        std::fs::create_dir(root.join("sub")).unwrap();
        std::fs::write(root.join("sub").join("three.rs"), b"// hello").unwrap();
        dir
    }

    fn among(query: &Query, candidates: Vec<VPath>) -> (Vec<String>, Stats) {
        let mut names = Vec::new();
        let stats = find_among(
            &LocalVfs,
            query,
            &mut candidates.into_iter(),
            &CancelToken::new(),
            &mut |event| {
                if let Event::Hit(hit) = event {
                    names.push(hit.path.display_name());
                }
            },
        )
        .unwrap();
        names.sort();
        (names, stats)
    }

    #[test]
    fn what_an_index_proposes_is_looked_at_again_and_asked_everything() {
        let dir = tree();
        let root = VPath::local(dir.path());
        let elsewhere = tempfile::tempdir().unwrap();
        std::fs::write(elsewhere.path().join("stray.rs"), b"x").unwrap();
        let proposed = vec![
            root.join("one.rs"),
            root.join("two.txt"),
            root.join("sub").join("three.rs"),
            root.join("sub"),
            // Named twice, as an index with two entries for one file would.
            root.join("one.rs"),
            // Gone since the index was written: left out, and not a problem.
            root.join("deleted.rs"),
            // Not under the folder that was asked about, whatever the index thinks.
            VPath::local(elsewhere.path().join("stray.rs")),
            // The folder itself is never one of its own results.
            root.clone(),
        ];

        let query = Query::under(root.clone())
            .with_filter(Filter::from_masks(MaskSet::parse("*.rs").unwrap()));
        let (names, stats) = among(&query, proposed.clone());
        assert_eq!(names, ["one.rs", "three.rs"]);
        assert_eq!(stats.problems, 0);
        assert_eq!(stats.files_seen, 3);
        assert_eq!(stats.dirs_seen, 1);

        // "This folder" alone: what is deeper is not a result, though the index named it.
        let mut shallow = query.clone();
        shallow.subdirectories = false;
        assert_eq!(among(&shallow, proposed.clone()).0, ["one.rs"]);
        let mut one_deep = query.clone();
        one_deep.depth = Some(1);
        assert_eq!(among(&one_deep, proposed.clone()).0, ["one.rs"]);

        // The text condition reads the files, as it does on a walk.
        let with_text = Query::under(root.clone())
            .containing(crate::Content::containing(crate::Needle::text("hello")));
        assert_eq!(
            among(&with_text, proposed.clone()).0,
            ["three.rs", "two.txt"]
        );

        // Folders are results only where the query wants them.
        let mut dirs = Query::under(root);
        dirs.include_dirs = true;
        assert_eq!(
            among(&dirs, proposed).0,
            ["one.rs", "sub", "three.rs", "two.txt"]
        );
    }

    #[test]
    fn an_index_that_cannot_be_asked_says_so_and_does_not_pretend_to_have_found_nothing() {
        let dir = tree();
        let query = Query::under(VPath::local(dir.path()));
        let cancel = CancelToken::new();
        let run = |index: &Index, query: &Query| {
            find_indexed(&LocalVfs, query, index, "", &cancel, &mut |_| {})
        };
        // A program that is not there.
        let missing = Index {
            name: "nothing".into(),
            program: dir.path().join("no-such-program"),
            kind: Kind::Custom,
            template: Vec::new(),
        };
        assert!(matches!(
            run(&missing, &query),
            Err(IndexError::Unavailable(_))
        ));
        // Archives, and folders that are not on this machine's disk, are not its business.
        let mut inside = query.clone();
        inside.in_archives = true;
        assert!(matches!(
            run(&missing, &inside),
            Err(IndexError::Unavailable(_))
        ));
        let remote = Query::under(VPath::remote(fcmd_core::Scheme::Sftp, "mb@host:22", "/srv"));
        assert!(matches!(
            run(&missing, &remote),
            Err(IndexError::Unavailable(_))
        ));
    }

    /// The whole of it through a real program — the one program every machine has: a shell
    /// that prints a file. No index is needed to test that an index is listened to.
    #[test]
    fn a_configured_command_is_run_and_its_answer_searched() {
        let dir = tree();
        let root = VPath::local(dir.path());
        let list = dir.path().join("index.txt");
        let lines: Vec<String> = ["one.rs", "two.txt", "gone.rs"]
            .iter()
            .map(|name| root.join(name).as_path().to_string_lossy().into_owned())
            .collect();
        std::fs::write(&list, lines.join("\n")).unwrap();

        let command = if cfg!(windows) {
            format!("cmd /c type {}", list.display())
        } else {
            format!("cat {}", list.display())
        };
        let Some(index) = Index::custom(&command) else {
            // A machine with neither is not one this test has anything to say about.
            return;
        };
        let query = Query::under(root.clone())
            .with_filter(Filter::from_masks(MaskSet::parse("*.rs").unwrap()));
        let mut names = Vec::new();
        let stats = find_indexed(
            &LocalVfs,
            &query,
            &index,
            "",
            &CancelToken::new(),
            &mut |event| {
                if let Event::Hit(hit) = event {
                    names.push(hit.path.display_name());
                }
            },
        )
        .unwrap();
        assert_eq!(names, ["one.rs"]);
        assert_eq!(stats.files_seen, 2, "the file that is gone is not counted");

        // A program that fails and says why is an index that could not be asked.
        let broken = if cfg!(windows) {
            format!("cmd /c type {}", dir.path().join("no-such-list").display())
        } else {
            format!("cat {}", dir.path().join("no-such-list").display())
        };
        let index = Index::custom(&broken).unwrap();
        let result = find_indexed(
            &LocalVfs,
            &query,
            &index,
            "",
            &CancelToken::new(),
            &mut |_| {},
        );
        assert!(
            matches!(result, Err(IndexError::Unavailable(_))),
            "{result:?}"
        );
    }
}
