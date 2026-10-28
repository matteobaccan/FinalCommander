//! Changing what a file *is*, rather than what is in it.
//!
//! This is the other half of [`JobFs::apply_meta`], and the difference is who asked. A copy
//! applies the source's metadata as a courtesy, and swallows a failure rather than losing the
//! copy over it. Here somebody opened a dialog, ticked a box and pressed Enter: a permission
//! that could not be set has to be reported, with the path and the reason, or the dialog has
//! lied about what it did.
//!
//! What can be changed is what the platform has. Unix has a mode and two timestamps; Windows
//! has four attribute bits and the same timestamps. The two overlap in exactly one place —
//! read-only, which on Unix is the write bits and on Windows is a flag of its own — so that is
//! the one offered on both and the rest are offered where they exist.

use std::time::SystemTime;

use fcmd_core::{CancelToken, VPath};

use crate::fs::JobFs;
use crate::run::Outcome;

/// What to change. `None` leaves that thing exactly as it is, which is what every row of the
/// dialog the user did not touch means.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Change {
    /// The whole Unix mode, assembled from the dialog's twelve boxes. Ignored on Windows,
    /// which has no mode to set.
    pub mode: Option<u32>,
    /// Read-only: the write bits on Unix, the attribute on Windows. Applied after `mode`, so
    /// a dialog that offers both ends with the answer the user gave last.
    pub readonly: Option<bool>,
    /// Windows' three other bits. Nothing on Unix has them, and the dialog does not offer
    /// them there.
    pub hidden: Option<bool>,
    pub system: Option<bool>,
    pub archive: Option<bool>,
    /// The three more that `SetFileAttributes` will take: whether the content indexer looks
    /// at it, whether it is meant to be short-lived, and whether its contents live elsewhere.
    /// The rest of Windows' bits — compressed, encrypted, sparse, a reparse point — are set
    /// by doing the thing rather than by writing a bit, so the dialog shows them and does not
    /// offer them.
    pub indexed: Option<bool>,
    pub temporary: Option<bool>,
    pub offline: Option<bool>,
    pub mtime: Option<SystemTime>,
    pub atime: Option<SystemTime>,
    /// When it was made. Only Windows keeps one that can be set; everywhere else the row is
    /// shown and not offered, because there is no call to write it with.
    pub btime: Option<SystemTime>,
}

impl Change {
    /// Is there anything to do? A dialog closed with nothing touched should not walk a tree.
    pub fn is_empty(&self) -> bool {
        *self == Change::default()
    }
}

/// Apply `change` to each of `paths`, and to everything under them when `recursive`.
///
/// Every path is reported through `progress` before it is touched, so the box on screen names
/// what is being changed rather than only counting. A path that refuses is collected and the
/// walk goes on: one file in a tree that somebody else owns should not stop the other hundred.
pub fn set_attributes(
    fs: &dyn JobFs,
    paths: &[VPath],
    change: &Change,
    recursive: bool,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(&VPath, u64),
) -> Outcome {
    let mut outcome = Outcome::default();
    if change.is_empty() {
        return outcome;
    }
    for path in paths {
        walk(fs, path, change, recursive, cancel, progress, &mut outcome);
        if outcome.cancelled {
            break;
        }
    }
    outcome
}

/// One path, and its tree below it when asked.
///
/// The directory itself is changed *after* what is inside it. Taking the write bit off a
/// directory first would make its own contents unreachable, which is the one ordering mistake
/// this walk can make and the only reason it is written out rather than left to recursion's
/// natural order.
fn walk(
    fs: &dyn JobFs,
    path: &VPath,
    change: &Change,
    recursive: bool,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(&VPath, u64),
    outcome: &mut Outcome,
) {
    if cancel.is_cancelled() {
        outcome.cancelled = true;
        return;
    }
    let facts = match fs.facts(path) {
        Ok(facts) => facts,
        Err(e) => {
            outcome.errors.push((path.clone(), e.to_string()));
            return;
        }
    };
    if recursive && facts.is_dir() {
        match fs.read_dir(path) {
            Ok(rows) => {
                for (name, _) in rows {
                    let child = path.join(name.to_string_lossy().as_ref());
                    walk(fs, &child, change, recursive, cancel, progress, outcome);
                    if outcome.cancelled {
                        return;
                    }
                }
            }
            Err(e) => outcome.errors.push((path.clone(), e.to_string())),
        }
    }
    progress(path, outcome.files + outcome.dirs);
    match fs.set_attrs(path, change) {
        Ok(()) => {
            if facts.is_dir() {
                outcome.dirs += 1;
            } else {
                outcome.files += 1;
            }
        }
        Err(e) => outcome.errors.push((path.clone(), e.to_string())),
    }
}

/// The Unix mode a set of twelve booleans means, and the booleans a mode means — the two
/// halves of the dialog, in one place so they cannot disagree.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Mode {
    pub owner_read: bool,
    pub owner_write: bool,
    pub owner_exec: bool,
    pub group_read: bool,
    pub group_write: bool,
    pub group_exec: bool,
    pub other_read: bool,
    pub other_write: bool,
    pub other_exec: bool,
    pub setuid: bool,
    pub setgid: bool,
    pub sticky: bool,
}

impl Mode {
    /// The twelve boxes a mode ticks.
    pub fn of(mode: u32) -> Mode {
        Mode {
            owner_read: mode & 0o400 != 0,
            owner_write: mode & 0o200 != 0,
            owner_exec: mode & 0o100 != 0,
            group_read: mode & 0o040 != 0,
            group_write: mode & 0o020 != 0,
            group_exec: mode & 0o010 != 0,
            other_read: mode & 0o004 != 0,
            other_write: mode & 0o002 != 0,
            other_exec: mode & 0o001 != 0,
            setuid: mode & 0o4000 != 0,
            setgid: mode & 0o2000 != 0,
            sticky: mode & 0o1000 != 0,
        }
    }

    /// The mode twelve boxes mean. Only the permission bits: the file type bits of a `st_mode`
    /// are not a permission and are not this dialog's to change.
    pub fn bits(self) -> u32 {
        let mut mode = 0u32;
        for (on, bit) in [
            (self.owner_read, 0o400),
            (self.owner_write, 0o200),
            (self.owner_exec, 0o100),
            (self.group_read, 0o040),
            (self.group_write, 0o020),
            (self.group_exec, 0o010),
            (self.other_read, 0o004),
            (self.other_write, 0o002),
            (self.other_exec, 0o001),
            (self.setuid, 0o4000),
            (self.setgid, 0o2000),
            (self.sticky, 0o1000),
        ] {
            if on {
                mode |= bit;
            }
        }
        mode
    }

    /// `rwxr-xr-x`, as `ls` writes it and as the panel's attribute column already does.
    pub fn text(self) -> String {
        let bit = |on: bool, c: char| if on { c } else { '-' };
        let special = |on: bool, exec: bool, lower: char, upper: char| match (on, exec) {
            (true, true) => lower,
            (true, false) => upper,
            (false, true) => 'x',
            (false, false) => '-',
        };
        [
            bit(self.owner_read, 'r'),
            bit(self.owner_write, 'w'),
            special(self.setuid, self.owner_exec, 's', 'S'),
            bit(self.group_read, 'r'),
            bit(self.group_write, 'w'),
            special(self.setgid, self.group_exec, 's', 'S'),
            bit(self.other_read, 'r'),
            bit(self.other_write, 'w'),
            special(self.sticky, self.other_exec, 't', 'T'),
        ]
        .into_iter()
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::{Facts, LocalFs};
    use fcmd_core::EntryKind;

    #[test]
    fn a_mode_survives_the_round_trip_through_twelve_boxes() {
        for mode in [0o644, 0o755, 0o600, 0o777, 0o000, 0o4755, 0o1777, 0o2750] {
            assert_eq!(Mode::of(mode).bits(), mode, "{mode:o}");
        }
    }

    #[test]
    fn the_text_reads_as_ls_writes_it() {
        assert_eq!(Mode::of(0o644).text(), "rw-r--r--");
        assert_eq!(Mode::of(0o755).text(), "rwxr-xr-x");
        assert_eq!(Mode::of(0o000).text(), "---------");
        // The special bits take the place of the execute bit they qualify, in upper case when
        // the execute bit they sit on is not set — which is how `ls` says "this is probably a
        // mistake".
        assert_eq!(Mode::of(0o4755).text(), "rwsr-xr-x");
        assert_eq!(Mode::of(0o4655).text(), "rwSr-xr-x");
        assert_eq!(Mode::of(0o1777).text(), "rwxrwxrwt");
    }

    #[test]
    fn nothing_asked_for_is_nothing_done() {
        let change = Change::default();
        assert!(change.is_empty());
        let dir = tempfile::tempdir().unwrap();
        let outcome = set_attributes(
            &LocalFs,
            &[VPath::local(dir.path())],
            &change,
            true,
            &CancelToken::new(),
            &mut |_, _| {},
        );
        assert_eq!(outcome.files, 0);
        assert_eq!(outcome.dirs, 0);
        assert!(outcome.errors.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn a_mode_is_set_on_a_file_and_on_a_whole_tree() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir(root.join("inside")).unwrap();
        std::fs::write(root.join("one.txt"), b"x").unwrap();
        std::fs::write(root.join("inside/two.txt"), b"y").unwrap();

        let mode_of =
            |p: &std::path::Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o7777;

        // One file, and nothing else touched.
        let change = Change {
            mode: Some(0o600),
            ..Change::default()
        };
        let outcome = set_attributes(
            &LocalFs,
            &[VPath::local(root.join("one.txt"))],
            &change,
            false,
            &CancelToken::new(),
            &mut |_, _| {},
        );
        assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);
        assert_eq!(outcome.files, 1);
        assert_eq!(mode_of(&root.join("one.txt")), 0o600);
        assert_ne!(mode_of(&root.join("inside/two.txt")), 0o600);

        // The whole tree, directories included — and a directory keeps its execute bit or
        // nothing inside it could be reached, which is the caller's business and not this
        // function's: it sets what it was told to set.
        let change = Change {
            mode: Some(0o750),
            ..Change::default()
        };
        let outcome = set_attributes(
            &LocalFs,
            &[VPath::local(root)],
            &change,
            true,
            &CancelToken::new(),
            &mut |_, _| {},
        );
        assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);
        assert_eq!(outcome.files, 2, "both files");
        assert_eq!(outcome.dirs, 2, "the root and the one inside it");
        assert_eq!(mode_of(&root.join("inside/two.txt")), 0o750);
        assert_eq!(mode_of(&root.join("inside")), 0o750);
    }

    #[cfg(unix)]
    #[test]
    fn read_only_is_the_write_bits_and_nothing_else() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("f.txt");
        std::fs::write(&file, b"x").unwrap();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o664)).unwrap();
        let mode_of = || std::fs::metadata(&file).unwrap().permissions().mode() & 0o7777;

        let on = Change {
            readonly: Some(true),
            ..Change::default()
        };
        set_attributes(
            &LocalFs,
            &[VPath::local(&file)],
            &on,
            false,
            &CancelToken::new(),
            &mut |_, _| {},
        );
        assert_eq!(mode_of(), 0o444, "every write bit goes, the rest stays");

        let off = Change {
            readonly: Some(false),
            ..Change::default()
        };
        set_attributes(
            &LocalFs,
            &[VPath::local(&file)],
            &off,
            false,
            &CancelToken::new(),
            &mut |_, _| {},
        );
        // Writable again, for whoever the read bits already allowed.
        assert_eq!(mode_of(), 0o644);
    }

    #[test]
    fn a_time_can_be_set_and_a_failure_is_reported_rather_than_swallowed() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("f.txt");
        std::fs::write(&file, b"x").unwrap();
        let when = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_000);

        let change = Change {
            mtime: Some(when),
            ..Change::default()
        };
        let outcome = set_attributes(
            &LocalFs,
            &[VPath::local(&file)],
            &change,
            false,
            &CancelToken::new(),
            &mut |_, _| {},
        );
        assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);
        let facts = LocalFs.facts(&VPath::local(&file)).unwrap();
        assert_eq!(facts.mtime, Some(when));

        // A path that is not there is an error with the path in it, not a silent nothing.
        let outcome = set_attributes(
            &LocalFs,
            &[VPath::local(dir.path().join("nope"))],
            &change,
            false,
            &CancelToken::new(),
            &mut |_, _| {},
        );
        assert_eq!(outcome.errors.len(), 1);
        assert!(outcome.errors[0].0.to_string().ends_with("nope"));
    }

    #[test]
    fn the_walk_stops_when_it_is_cancelled() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..20 {
            std::fs::write(dir.path().join(format!("{i}.txt")), b"x").unwrap();
        }
        let cancel = CancelToken::new();
        let change = Change {
            mtime: Some(SystemTime::UNIX_EPOCH),
            ..Change::default()
        };
        let mut seen = 0u64;
        let outcome = set_attributes(
            &LocalFs,
            &[VPath::local(dir.path())],
            &change,
            true,
            &cancel,
            &mut |_, _| {
                seen += 1;
                if seen == 3 {
                    cancel.cancel();
                }
            },
        );
        assert!(outcome.cancelled);
        assert!(outcome.files < 20, "it stopped early: {}", outcome.files);
    }

    /// A provider that cannot change a thing says so, and the dialog shows what it said.
    #[test]
    fn a_filesystem_with_nothing_to_set_refuses() {
        struct Bare;
        impl JobFs for Bare {
            fn facts(&self, _p: &VPath) -> std::io::Result<Facts> {
                Ok(Facts {
                    kind: EntryKind::File,
                    size: 0,
                    mtime: None,
                    mode: None,
                    readonly: false,
                })
            }
            fn facts_following(&self, p: &VPath) -> std::io::Result<Facts> {
                self.facts(p)
            }
            fn exists(&self, _p: &VPath) -> bool {
                true
            }
            fn read_dir(&self, _p: &VPath) -> std::io::Result<Vec<(std::ffi::OsString, Facts)>> {
                Ok(Vec::new())
            }
            fn create_dir(&self, _p: &VPath) -> std::io::Result<()> {
                Ok(())
            }
            fn create_dir_all(&self, _p: &VPath) -> std::io::Result<()> {
                Ok(())
            }
            fn rename(&self, _a: &VPath, _b: &VPath) -> std::io::Result<()> {
                Ok(())
            }
            fn remove_file(&self, _p: &VPath) -> std::io::Result<()> {
                Ok(())
            }
            fn remove_dir(&self, _p: &VPath) -> std::io::Result<()> {
                Ok(())
            }
            fn read_link(&self, _p: &VPath) -> std::io::Result<std::path::PathBuf> {
                Ok(std::path::PathBuf::new())
            }
            fn symlink(&self, _t: &std::path::Path, _l: &VPath) -> std::io::Result<()> {
                Ok(())
            }
            fn same_file(&self, _a: &VPath, _b: &VPath) -> bool {
                false
            }
            fn apply_meta(
                &self,
                _p: &VPath,
                _f: &Facts,
                _pr: crate::Preserve,
            ) -> std::io::Result<()> {
                Ok(())
            }
            fn copy_file(
                &self,
                _s: &VPath,
                _d: &VPath,
                _b: usize,
                _o: &mut dyn FnMut(u64) -> bool,
            ) -> std::io::Result<u64> {
                Ok(0)
            }
        }

        let change = Change {
            mode: Some(0o600),
            ..Change::default()
        };
        let outcome = set_attributes(
            &Bare,
            &[VPath::parse("s3://bucket/key")],
            &change,
            false,
            &CancelToken::new(),
            &mut |_, _| {},
        );
        assert_eq!(outcome.files, 0);
        assert_eq!(outcome.errors.len(), 1);
        assert!(
            outcome.errors[0].1.contains("cannot"),
            "{}",
            outcome.errors[0].1
        );
    }
}
