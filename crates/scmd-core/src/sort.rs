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
        // The bytes of the name, not a `str` made from them. `to_string_lossy` validates the
        // whole name as UTF-8 on every call, and this is called twice per comparison and some
        // thirty-five million times for a panel of 1.7 million rows. The byte path reaches a
        // character only where one of the two names actually holds a byte above ASCII at the
        // point the comparison turns on, which for a tree of file names is almost never.
        if self.natural {
            natural_cmp_bytes(
                a.name.as_encoded_bytes(),
                b.name.as_encoded_bytes(),
                self.case_sensitive,
            )
        } else {
            self.cmp_str(&a.name_str(), &b.name_str())
        }
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
///
/// Two names that begin alike are settled by what comes after, so the shared head is skipped
/// with a byte comparison the machine does a word at a time instead of walking it character by
/// character. That is not an optimisation of the obvious kind: a panel over a bucket or a build
/// tree holds names that agree for their first thirty characters
/// (`2024/08/13/customer-00012345/…`), and walking those thirty twice per comparison, times
/// the thirty-five million comparisons a sort of 1.7 million rows performs, is a lot of
/// identical prefix.
///
/// Two things make the skip safe. The point where the names diverge may sit inside a run of
/// digits — `a2x` against `a21` diverges at `x` against `1`, and the answer depends on 2
/// against 21, not on `x` against `1` — so the cursor walks back to the start of that run.
/// And it may sit inside a multi-byte character, whose leading bytes two different characters
/// can share, so it walks back to the character boundary. From there the comparison proceeds
/// exactly as it always did.
pub fn natural_cmp(a: &str, b: &str, case_sensitive: bool) -> Ordering {
    natural_cmp_bytes(a.as_bytes(), b.as_bytes(), case_sensitive)
}

/// The same comparison on the bytes of a name, which is how a listing holds it.
///
/// A file name is not always a `str`: on Unix it is any bytes without a slash or a zero, on
/// Windows it is UTF-16 that need not be well formed. Comparing the encoded bytes is both
/// faster — no name is validated as UTF-8 to be sorted — and truer to the name, because two
/// names that are not UTF-8 no longer collapse onto the same replacement character. UTF-8
/// orders its bytes the way Unicode orders its characters, so for a case-sensitive comparison
/// the bytes are the answer; only folding case outside ASCII needs the characters themselves,
/// and that is the one path that decodes.
pub fn natural_cmp_bytes(a: &[u8], b: &[u8], case_sensitive: bool) -> Ordering {
    let common = a.len().min(b.len());
    let mut i = 0;
    while i < common && a[i] == b[i] {
        i += 1;
    }
    if i == common {
        // One name is the other with more after it, or they are the same name.
        return a.len().cmp(&b.len());
    }
    while i > 0 && a[i - 1].is_ascii_digit() {
        i -= 1;
    }
    // A continuation byte cannot begin a character, and both names are inside the same
    // character here, since every byte before this one was equal.
    while i > 0 && a[i] & 0xC0 == 0x80 {
        i -= 1;
    }
    natural_cmp_from(&a[i..], &b[i..], case_sensitive)
}

/// The comparison itself, on the part of the two names that can still decide it.
fn natural_cmp_from(a: &[u8], b: &[u8], case_sensitive: bool) -> Ordering {
    // `img02` and `img2` are two different files and must not compare equal, or the order
    // between them is whatever the sort happened to do. Numerically those runs are the same,
    // so the padding decides — but last, after everything else has had its say, which is what
    // keeps `a2b` before `a02c`.
    let mut padding = Ordering::Equal;
    let (mut x, mut y) = (0usize, 0usize);
    loop {
        match (a.get(x).copied(), b.get(y).copied()) {
            (None, None) => return padding,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(ca), Some(cb)) => {
                if ca.is_ascii_digit() && cb.is_ascii_digit() {
                    let (na, la) = take_number(a, &mut x);
                    let (nb, lb) = take_number(b, &mut y);
                    match na.cmp(&nb) {
                        Ordering::Equal => {
                            if padding == Ordering::Equal {
                                padding = la.cmp(&lb);
                            }
                        }
                        other => return other,
                    }
                } else if case_sensitive || (ca.is_ascii() && cb.is_ascii()) {
                    x += 1;
                    y += 1;
                    let (ka, kb) = if case_sensitive {
                        (ca, cb)
                    } else {
                        (ca.to_ascii_lowercase(), cb.to_ascii_lowercase())
                    };
                    match ka.cmp(&kb) {
                        Ordering::Equal => {}
                        other => return other,
                    }
                } else {
                    // Folding case outside ASCII is a question about characters, not bytes.
                    // This is the only path that decodes, and it starts at a character
                    // boundary: every byte before it was equal or was ASCII.
                    let rest = natural_cmp_chars(&a[x..], &b[y..]);
                    return if rest == Ordering::Equal {
                        padding
                    } else {
                        rest
                    };
                }
            }
        }
    }
}

/// The tail of a case-insensitive comparison where one of the two names left ASCII behind.
///
/// Same rules as the byte loop, characters instead of bytes: runs of digits are still numbers,
/// and padding still decides last. Folding is the only reason to be here, so it is the only
/// thing done differently.
fn natural_cmp_chars(a: &[u8], b: &[u8]) -> Ordering {
    let (a, b) = (String::from_utf8_lossy(a), String::from_utf8_lossy(b));
    let mut ai = a.chars().peekable();
    let mut bi = b.chars().peekable();
    let mut padding = Ordering::Equal;
    loop {
        match (ai.peek().copied(), bi.peek().copied()) {
            (None, None) => return padding,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(ca), Some(cb)) => {
                if ca.is_ascii_digit() && cb.is_ascii_digit() {
                    let (na, la) = take_number_chars(&mut ai);
                    let (nb, lb) = take_number_chars(&mut bi);
                    match na.cmp(&nb) {
                        Ordering::Equal => {
                            if padding == Ordering::Equal {
                                padding = la.cmp(&lb);
                            }
                        }
                        other => return other,
                    }
                } else {
                    ai.next();
                    bi.next();
                    let ka = ca.to_lowercase().next().unwrap_or(ca);
                    let kb = cb.to_lowercase().next().unwrap_or(cb);
                    match ka.cmp(&kb) {
                        Ordering::Equal => {}
                        other => return other,
                    }
                }
            }
        }
    }
}

/// [`take_number`] over characters, for the folding path.
fn take_number_chars(it: &mut std::iter::Peekable<std::str::Chars<'_>>) -> (u128, usize) {
    let mut n: u128 = 0;
    let mut digits = 0usize;
    while let Some(c) = it.peek().copied() {
        if !c.is_ascii_digit() {
            break;
        }
        it.next();
        digits += 1;
        n = n
            .saturating_mul(10)
            .saturating_add((c as u8 - b'0') as u128);
    }
    (n, digits)
}

/// The number a run of digits spells, and how many digits spelled it.
fn take_number(bytes: &[u8], at: &mut usize) -> (u128, usize) {
    let mut n: u128 = 0;
    let mut digits = 0usize;
    while let Some(c) = bytes.get(*at).copied() {
        if !c.is_ascii_digit() {
            break;
        }
        *at += 1;
        digits += 1;
        n = n.saturating_mul(10).saturating_add((c - b'0') as u128);
    }
    (n, digits)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shared-head skip must be invisible: whatever it decides, the comparison that walks
    /// every character from the first must decide the same.
    ///
    /// The pairs are built to land on the cases the skip has to be careful about — a
    /// divergence inside a run of digits, leading zeros, a digit run that runs to the end of
    /// one name, a multi-byte character whose lead byte another one shares, one name a prefix
    /// of the other — and then every pair is tried in both directions and in both cases.
    #[test]
    fn the_shared_head_skip_decides_what_the_long_way_decides() {
        let parts = [
            "",
            "a",
            "A",
            "a2",
            "a02",
            "a2x",
            "a21",
            "a210",
            "a2x1",
            "0",
            "00",
            "1",
            "10",
            "9",
            "img",
            "img2",
            "img10",
            "img02",
            "x",
            "è",
            "é",
            "日",
            "日本",
            "f1b",
            "f1B",
            "f01b",
            "2024/08/13/x",
            "2024/08/13/y",
            "2024/08/3/x",
            "2024/8/13/x",
        ];
        let mut pairs = 0;
        for a in parts {
            for b in parts {
                for case_sensitive in [true, false] {
                    let fast = natural_cmp(a, b, case_sensitive);
                    let slow = natural_cmp_from(a.as_bytes(), b.as_bytes(), case_sensitive);
                    assert_eq!(
                        fast, slow,
                        "{a:?} vs {b:?} (case_sensitive = {case_sensitive}): \
                         the skip says {fast:?}, the long way says {slow:?}"
                    );
                    pairs += 1;
                }
            }
        }
        assert!(pairs > 1_000, "the table got smaller by accident: {pairs}");
    }

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

    /// Two names that differ only in how a number is padded are still two names.
    #[test]
    fn leading_zeros_decide_last_but_they_do_decide() {
        // Not equal: they are different files, and an order that calls them equal leaves the
        // two rows wherever the sort dropped them.
        assert_eq!(natural_cmp("img02", "img2", true), Ordering::Greater);
        assert_eq!(natural_cmp("0", "00", true), Ordering::Less);
        assert_eq!(natural_cmp("00", "0", true), Ordering::Greater);
        // Last, though: what follows the number has the first word, so the padding only
        // settles the pair that is otherwise the same name.
        assert_eq!(natural_cmp("a2b", "a02c", true), Ordering::Less);
        assert_eq!(natural_cmp("a2x", "a02x", true), Ordering::Less);
        assert_eq!(natural_cmp("a02x", "a2x", true), Ordering::Greater);
        // And the number itself still comes before either of them.
        assert_eq!(natural_cmp("a03", "a2", true), Ordering::Greater);
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
