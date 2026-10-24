use std::cmp::Ordering;

use crate::entry::{Entry, EntryKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SortKey {
    Name,
    Ext,
    Size,
    MTime,
    /// Creation time (`Ctrl+F10` in FAR's order) and last access time (`Ctrl+F11`).
    BTime,
    ATime,
    Unsorted,
}

impl SortKey {
    pub fn as_str(self) -> &'static str {
        match self {
            SortKey::Name => "name",
            SortKey::Ext => "ext",
            SortKey::Size => "size",
            SortKey::MTime => "mtime",
            SortKey::BTime => "btime",
            SortKey::ATime => "atime",
            SortKey::Unsorted => "none",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sort {
    pub key: SortKey,
    pub reverse: bool,
    pub dirs_first: bool,
    pub selected_first: bool,
    /// Treat digit runs as numbers: `file2` before `file10`.
    pub natural: bool,
    pub case_sensitive: bool,
}

impl Default for Sort {
    fn default() -> Self {
        Sort {
            key: SortKey::Name,
            reverse: false,
            dirs_first: true,
            selected_first: false,
            natural: true,
            case_sensitive: cfg!(not(any(windows, target_os = "macos"))),
        }
    }
}

impl Sort {
    /// Pressing the same sort key twice flips the direction (FAR's `allow_reverse_sort`).
    pub fn toggle(&mut self, key: SortKey, allow_reverse: bool) {
        if self.key == key && allow_reverse {
            self.reverse = !self.reverse;
        } else {
            self.key = key;
            self.reverse = false;
        }
    }

    pub fn compare(&self, a: &Entry, b: &Entry) -> Ordering {
        // `..` always first, whatever the sort.
        match (a.kind == EntryKind::Parent, b.kind == EntryKind::Parent) {
            (true, true) => return Ordering::Equal,
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            _ => {}
        }
        if self.dirs_first {
            match (a.kind.is_dirlike(), b.kind.is_dirlike()) {
                (true, false) => return Ordering::Less,
                (false, true) => return Ordering::Greater,
                _ => {}
            }
        }

        let primary = match self.key {
            SortKey::Name => self.cmp_name(a, b),
            SortKey::Ext => self
                .cmp_str(&a.extension(), &b.extension())
                .then_with(|| self.cmp_name(a, b)),
            SortKey::Size => a.size.cmp(&b.size).then_with(|| self.cmp_name(a, b)),
            SortKey::MTime => a.mtime.cmp(&b.mtime).then_with(|| self.cmp_name(a, b)),
            // A timestamp the filesystem does not keep counts as older than any it does, so
            // the entries nobody can date gather at the old end of the order and `reverse`
            // carries them along with everything else. That is `Option`'s own ordering, which
            // is what the write-time key has always done; the name breaks the remaining ties,
            // so the comparison is total and the order stable whatever is missing.
            SortKey::BTime => a.btime.cmp(&b.btime).then_with(|| self.cmp_name(a, b)),
            SortKey::ATime => a.atime.cmp(&b.atime).then_with(|| self.cmp_name(a, b)),
            SortKey::Unsorted => Ordering::Equal,
        };

        if self.reverse && self.key != SortKey::Unsorted {
            primary.reverse()
        } else {
            primary
        }
    }

    fn cmp_name(&self, a: &Entry, b: &Entry) -> Ordering {
        self.cmp_str(&a.name_str(), &b.name_str())
    }

    fn cmp_str(&self, a: &str, b: &str) -> Ordering {
        if self.natural {
            natural_cmp(a, b, self.case_sensitive)
        } else if self.case_sensitive {
            a.cmp(b)
        } else {
            a.to_lowercase().cmp(&b.to_lowercase())
        }
    }
}

/// Compare treating digit runs as numbers. `img2` < `img10`, `a` == `A` when insensitive.
pub fn natural_cmp(a: &str, b: &str, case_sensitive: bool) -> Ordering {
    let mut ai = a.chars().peekable();
    let mut bi = b.chars().peekable();
    loop {
        match (ai.peek().copied(), bi.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(ca), Some(cb)) => {
                if ca.is_ascii_digit() && cb.is_ascii_digit() {
                    let na = take_number(&mut ai);
                    let nb = take_number(&mut bi);
                    match na.cmp(&nb) {
                        Ordering::Equal => {}
                        other => return other,
                    }
                } else {
                    ai.next();
                    bi.next();
                    let (ka, kb) = if case_sensitive {
                        (ca, cb)
                    } else {
                        (
                            ca.to_lowercase().next().unwrap_or(ca),
                            cb.to_lowercase().next().unwrap_or(cb),
                        )
                    };
                    match ka.cmp(&kb) {
                        Ordering::Equal => {}
                        other => return other,
                    }
                }
            }
        }
    }
}

fn take_number(it: &mut std::iter::Peekable<std::str::Chars<'_>>) -> u128 {
    let mut n: u128 = 0;
    while let Some(c) = it.peek().copied() {
        if !c.is_ascii_digit() {
            break;
        }
        it.next();
        n = n
            .saturating_mul(10)
            .saturating_add((c as u8 - b'0') as u128);
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::{Entry, EntryFlags, EntryId, EntryKind};
    use std::ffi::OsString;

    fn e(name: &str, kind: EntryKind, size: u64) -> Entry {
        Entry {
            name: OsString::from(name),
            kind,
            size,
            mtime: None,
            btime: None,
            atime: None,
            flags: EntryFlags::empty(),
            id: EntryId::Name(name.len() as u64),
            link_target: None,
        }
    }

    /// A file whose creation and access times are given in seconds since the epoch, with
    /// `None` standing for a filesystem that does not keep that field.
    fn timed(name: &str, btime: Option<u64>, atime: Option<u64>) -> Entry {
        let at = |secs: Option<u64>| {
            secs.map(|s| std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(s))
        };
        Entry {
            btime: at(btime),
            atime: at(atime),
            ..e(name, EntryKind::File, 0)
        }
    }

    fn names(mut v: Vec<Entry>, sort: Sort) -> Vec<String> {
        v.sort_by(|a, b| sort.compare(a, b));
        v.into_iter().map(|e| e.name_str().into_owned()).collect()
    }

    #[test]
    fn natural_order_beats_lexicographic() {
        assert_eq!(natural_cmp("img2", "img10", true), Ordering::Less);
        assert_eq!(natural_cmp("img10", "img2", true), Ordering::Greater);
        assert_eq!(natural_cmp("a", "A", false), Ordering::Equal);
    }

    #[test]
    fn parent_first_then_dirs() {
        let v = vec![
            e("zeta.txt", EntryKind::File, 1),
            e("src", EntryKind::Dir, 0),
            Entry::parent_entry(),
            e("alpha.txt", EntryKind::File, 2),
        ];
        assert_eq!(
            names(v, Sort::default()),
            vec!["..", "src", "alpha.txt", "zeta.txt"]
        );
    }

    #[test]
    fn reverse_keeps_parent_and_dirs_in_place() {
        let sort = Sort {
            reverse: true,
            ..Sort::default()
        };
        let v = vec![
            e("a.txt", EntryKind::File, 1),
            e("b.txt", EntryKind::File, 2),
            Entry::parent_entry(),
            e("dir", EntryKind::Dir, 0),
        ];
        assert_eq!(names(v, sort), vec!["..", "dir", "b.txt", "a.txt"]);
    }

    #[test]
    fn toggle_flips_only_on_the_same_key() {
        let mut s = Sort::default();
        s.toggle(SortKey::Size, true);
        assert_eq!((s.key, s.reverse), (SortKey::Size, false));
        s.toggle(SortKey::Size, true);
        assert_eq!((s.key, s.reverse), (SortKey::Size, true));
        s.toggle(SortKey::Name, true);
        assert_eq!((s.key, s.reverse), (SortKey::Name, false));
    }

    #[test]
    fn sort_by_creation_time_groups_the_undated_entries_at_the_old_end() {
        let v = vec![
            timed("new", Some(2_000), None),
            timed("unknown-b", None, None),
            timed("old", Some(1_000), None),
            timed("unknown-a", None, None),
        ];
        let sort = Sort {
            key: SortKey::BTime,
            ..Sort::default()
        };
        // Undated first, among themselves by name; reversed, the same group goes last.
        assert_eq!(
            names(v.clone(), sort),
            vec!["unknown-a", "unknown-b", "old", "new"]
        );
        let reversed = Sort {
            reverse: true,
            ..sort
        };
        assert_eq!(
            names(v, reversed),
            vec!["new", "old", "unknown-b", "unknown-a"]
        );
    }

    #[test]
    fn sort_by_access_time_is_total_even_when_every_entry_lacks_one() {
        let v = vec![
            timed("c", Some(9), None),
            timed("a", Some(1), None),
            timed("b", Some(5), None),
        ];
        let sort = Sort {
            key: SortKey::ATime,
            ..Sort::default()
        };
        // None of them has an access time, so the name decides and nothing is lost.
        assert_eq!(names(v, sort), vec!["a", "b", "c"]);

        let v = vec![
            timed("x", None, Some(2)),
            timed("y", None, Some(1)),
            timed("z", None, None),
        ];
        assert_eq!(names(v, sort), vec!["z", "y", "x"]);
    }

    #[test]
    fn every_sort_key_has_its_column_spelling() {
        assert_eq!(SortKey::BTime.as_str(), "btime");
        assert_eq!(SortKey::ATime.as_str(), "atime");
        assert_eq!(SortKey::MTime.as_str(), "mtime");
    }

    #[test]
    fn sort_by_size_then_name() {
        let v = vec![
            e("b", EntryKind::File, 10),
            e("a", EntryKind::File, 10),
            e("c", EntryKind::File, 5),
        ];
        let sort = Sort {
            key: SortKey::Size,
            ..Sort::default()
        };
        assert_eq!(names(v, sort), vec!["c", "a", "b"]);
    }
}
