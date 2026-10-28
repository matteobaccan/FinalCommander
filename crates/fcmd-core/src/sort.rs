use std::cmp::Ordering;

use crate::entry::{Entry, EntryKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SortKey {
    Name,
    Ext,
    Size,
    MTime,
    /// Creation time (`Ctrl+F8`) and last access time (`Ctrl+F9`), as FAR numbers them.
    BTime,
    ATime,
    /// What this directory's `descript.ion` says about the file (`Ctrl+F10`), and who the
    /// file belongs to (`Ctrl+F11`). Neither is in the entry: both are read by a worker and
    /// handed to the comparison as [`Side`], and a row nothing has answered for yet sorts
    /// with the ones that have nothing to say.
    Descr,
    Owner,
    Unsorted,
}

/// The two fields a sort key may ask for that are not in the entry itself.
///
/// They come from the maps the panel keeps beside the listing — the `descript.ion` it read
/// and the owners a worker looked up — because both cost a lookup per row and neither belongs
/// in an `Entry` that a million of exist. Empty means "nobody has said", which is a third
/// answer rather than an empty string: it sorts after everything that has one.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Side<'a> {
    pub descr: &'a str,
    pub owner: &'a str,
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
            SortKey::Descr => "descr",
            SortKey::Owner => "owner",
            SortKey::Unsorted => "none",
        }
    }
}

/// FAR's sort groups: masks that pull some files in front of the order and push others
/// behind it, whatever the sort key is.
///
/// It is what makes `*.cpp` come before `*.o` in a build directory however the panel is
/// sorted — a thing no sort key can say, because it is about *kinds* of file and not about
/// one field of them. Within a group the ordinary sort applies, so turning the groups on
/// does not take the sort away; it cuts the listing into bands and sorts inside each.
///
/// Directories are not grouped. They have their own place in the order already (`dirs_first`,
/// and `..` before everything), and a directory that happened to match `*.o` jumping into the
/// middle of the files is not what anybody wrote the mask for.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SortGroups {
    groups: Vec<SortGroup>,
}

/// One band: a name to show, the masks that fall into it, and which side of the ungrouped
/// files it sits on.
#[derive(Debug, Clone, PartialEq)]
pub struct SortGroup {
    pub name: String,
    pub masks: crate::filter::MaskSet,
    /// Behind the files that match no group, rather than in front of them.
    pub back: bool,
    /// Where this band sits: negative in front of the ungrouped files, positive behind them.
    /// Worked out when the list is built, so a comparison is one match and one `cmp`.
    rank: i32,
}

impl SortGroup {
    /// A band, before it knows where it sits: `SortGroups::new` works that out from the
    /// order the bands are given in.
    pub fn new(name: String, masks: crate::filter::MaskSet, back: bool) -> Self {
        SortGroup {
            name,
            masks,
            back,
            rank: 0,
        }
    }
}

impl SortGroups {
    /// The bands, in the order they were written. The ones in front keep that order among
    /// themselves, and so do the ones behind: the first group written is the first band.
    pub fn new(groups: Vec<SortGroup>) -> Self {
        let front = groups.iter().filter(|g| !g.back).count() as i32;
        let (mut ahead, mut behind) = (0, 0);
        let groups = groups
            .into_iter()
            .map(|mut g| {
                if g.back {
                    behind += 1;
                    g.rank = behind;
                } else {
                    ahead += 1;
                    g.rank = ahead - front - 1;
                }
                g
            })
            .collect();
        SortGroups { groups }
    }

    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }

    pub fn len(&self) -> usize {
        self.groups.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &SortGroup> {
        self.groups.iter()
    }

    /// Which band an entry falls in: the first group whose masks match it. Negative is in
    /// front of the ungrouped files, zero is among them, positive is behind them.
    pub fn rank(&self, entry: &Entry) -> i32 {
        self.group_of(entry).map(|g| g.rank).unwrap_or(0)
    }

    /// The band an entry falls in, for a panel that wants to say which.
    pub fn group_of(&self, entry: &Entry) -> Option<&SortGroup> {
        if entry.opens_as_dir() || entry.kind == EntryKind::Parent {
            return None;
        }
        let name = entry.name_str();
        self.groups.iter().find(|g| g.masks.matches_name(&name))
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
    /// Apply the sort groups (FAR's `Shift+F11`). The groups themselves live beside the
    /// panel rather than in here, because they are one list for the whole program and this
    /// is a copy per panel.
    pub groups: bool,
    /// The criteria chain: what breaks a tie the key leaves, in order, before the name does
    /// (SPEC 02 §4). Three slots rather than a list, because this type is `Copy` — a panel,
    /// a tab and a saved session each hold a copy — and because a fourth tie-breaker after
    /// three that all came out equal has nothing left to decide.
    pub chain: [Option<SortKey>; 3],
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
            groups: false,
            chain: [None; 3],
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

    /// Compare two entries with no sort groups in play.
    pub fn compare(&self, a: &Entry, b: &Entry) -> Ordering {
        self.compare_in(a, b, &SortGroups::default())
    }

    /// Compare two entries, the sort groups cutting the listing into bands first.
    pub fn compare_in(&self, a: &Entry, b: &Entry, groups: &SortGroups) -> Ordering {
        self.compare_side(a, b, groups, Side::default(), Side::default())
    }

    /// Does this sort ask for anything that is not in the entry? The panel reads it only when
    /// the answer is yes: a lookup per row for a field nobody is sorting by is a lookup for
    /// nothing.
    pub fn needs_side(&self) -> bool {
        matches!(self.key, SortKey::Descr | SortKey::Owner)
            || self.asks(SortKey::Descr)
            || self.asks(SortKey::Owner)
    }

    /// Is this key the one being sorted by, or one of the tie-breakers after it?
    pub fn asks(&self, want: SortKey) -> bool {
        self.key == want
            || (self.key != SortKey::Unsorted
                && self.chain.into_iter().flatten().any(|key| key == want))
    }

    /// The same comparison, with the two fields that live beside the listing handed in.
    pub fn compare_side(
        &self,
        a: &Entry,
        b: &Entry,
        groups: &SortGroups,
        sa: Side,
        sb: Side,
    ) -> Ordering {
        // `..` always first, whatever the sort.
        match (a.kind == EntryKind::Parent, b.kind == EntryKind::Parent) {
            (true, true) => return Ordering::Equal,
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            _ => {}
        }
        if self.dirs_first {
            // What opens as a directory sorts as one, so a junction to a folder is up with
            // the folders where somebody reading the listing looks for it.
            match (a.opens_as_dir(), b.opens_as_dir()) {
                (true, false) => return Ordering::Less,
                (false, true) => return Ordering::Greater,
                _ => {}
            }
        }

        // The bands, before the key: a group pulls its files out of the order altogether,
        // and the sort then applies inside each band. `reverse` does not touch them — it
        // turns the order round within a band, and a band that moved to the other end with
        // it would make the groups mean the opposite of what they were written to mean.
        if self.groups && !groups.is_empty() {
            let (ra, rb) = (groups.rank(a), groups.rank(b));
            if ra != rb {
                return ra.cmp(&rb);
            }
        }

        // `Unsorted` answers `Equal` to everything on purpose: the order the provider gave
        // is the order, and neither a chain nor a name may disturb it.
        if self.key == SortKey::Unsorted {
            return Ordering::Equal;
        }

        let mut ord = self.cmp_key(self.key, a, b, sa, sb);
        // The chain breaks the ties the key leaves: `["size", "name"]` sorts by size and
        // settles two files of the same size by name rather than by whatever came first.
        // A key already asked is skipped, because asking it twice cannot say anything new.
        if ord == Ordering::Equal {
            for key in self.chain.into_iter().flatten() {
                if key == self.key || key == SortKey::Unsorted {
                    continue;
                }
                ord = self.cmp_key(key, a, b, sa, sb);
                if ord != Ordering::Equal {
                    break;
                }
            }
        }
        // And the name has the last word whatever was asked, so the comparison is total and
        // the order does not wander between two reads of the same directory.
        if ord == Ordering::Equal {
            ord = self.cmp_name(a, b);
        }

        if self.reverse {
            ord.reverse()
        } else {
            ord
        }
    }

    /// One key's answer, with no tie-break of its own: what breaks the tie is the chain and
    /// then the name, decided once in `compare_in` rather than key by key.
    fn cmp_key(&self, key: SortKey, a: &Entry, b: &Entry, sa: Side, sb: Side) -> Ordering {
        match key {
            SortKey::Name => self.cmp_name(a, b),
            SortKey::Ext => self.cmp_str(&a.extension(), &b.extension()),
            SortKey::Size => a.size.cmp(&b.size),
            // A timestamp the filesystem does not keep counts as older than any it does, so
            // the entries nobody can date gather at the old end of the order and `reverse`
            // carries them along with everything else. That is `Option`'s own ordering, which
            // is what the write-time key has always done.
            SortKey::MTime => a.mtime.cmp(&b.mtime),
            SortKey::BTime => a.btime.cmp(&b.btime),
            SortKey::ATime => a.atime.cmp(&b.atime),
            // Nobody has said: after everything that has, rather than in with the empty
            // strings. A directory of a hundred files where three are described reads as
            // those three and then the rest, which is what the key is for.
            SortKey::Descr => self.cmp_side(sa.descr, sb.descr),
            SortKey::Owner => self.cmp_side(sa.owner, sb.owner),
            SortKey::Unsorted => Ordering::Equal,
        }
    }

    /// One of the fields from beside the listing: present before absent, and otherwise the
    /// same string comparison every other text key uses.
    fn cmp_side(&self, a: &str, b: &str) -> Ordering {
        match (a.is_empty(), b.is_empty()) {
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (false, false) => self.cmp_str(a, b),
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

    /// A band of the listing, by mask.
    fn band(name: &str, mask: &str, back: bool) -> SortGroup {
        SortGroup::new(
            name.to_string(),
            crate::filter::MaskSet::parse(mask).expect("a mask"),
            back,
        )
    }

    #[test]
    fn sort_groups_cut_the_listing_into_bands_and_the_sort_applies_inside_each() {
        let groups = SortGroups::new(vec![
            band("sources", "*.rs", false),
            band("headers", "*.h", false),
            band("output", "*.o,*.tmp", true),
        ]);
        let sort = Sort {
            groups: true,
            dirs_first: true,
            ..Sort::default()
        };
        let mut rows = [
            e("zeta.o", EntryKind::File, 0),
            e("beta.txt", EntryKind::File, 0),
            e("alpha.rs", EntryKind::File, 0),
            e("src", EntryKind::Dir, 0),
            e("one.h", EntryKind::File, 0),
            e("about.tmp", EntryKind::File, 0),
            e("zz.rs", EntryKind::File, 0),
        ];
        rows.sort_by(|a, b| sort.compare_in(a, b, &groups));
        let names: Vec<&str> = rows.iter().map(|e| e.name.to_str().unwrap()).collect();
        assert_eq!(
            names,
            [
                // The directory keeps its own place at the top; the bands are about files.
                "src",
                // The first band, then the second, in the order they were written — and
                // sorted by name inside each, which is the sort still doing its work.
                "alpha.rs",
                "zz.rs",
                "one.h",
                // Then what matched nothing.
                "beta.txt",
                // Then the band that was pushed behind.
                "about.tmp",
                "zeta.o",
            ]
        );

        // Turned off, the order is the plain one: the groups are a flag, not a rewrite.
        let plain = Sort {
            groups: false,
            ..sort
        };
        rows.sort_by(|a, b| plain.compare_in(a, b, &groups));
        let names: Vec<&str> = rows.iter().map(|e| e.name.to_str().unwrap()).collect();
        assert_eq!(
            names,
            [
                "src",
                "about.tmp",
                "alpha.rs",
                "beta.txt",
                "one.h",
                "zeta.o",
                "zz.rs"
            ]
        );
    }

    #[test]
    fn the_chain_breaks_the_ties_the_key_leaves_and_the_name_has_the_last_word() {
        let by_size = Sort {
            key: SortKey::Size,
            ..Sort::default()
        };
        let mut rows = [
            e("b.txt", EntryKind::File, 10),
            e("a.log", EntryKind::File, 10),
            e("c.log", EntryKind::File, 5),
        ];
        // With no chain, two files of the same size are settled by name, as they always were.
        rows.sort_by(|a, b| by_size.compare(a, b));
        let names: Vec<&str> = rows.iter().map(|e| e.name.to_str().unwrap()).collect();
        assert_eq!(names, ["c.log", "a.log", "b.txt"]);

        // With one, the tie goes to the extension first and only then to the name.
        let chained = Sort {
            chain: [Some(SortKey::Ext), None, None],
            ..by_size
        };
        rows.sort_by(|a, b| chained.compare(a, b));
        let names: Vec<&str> = rows.iter().map(|e| e.name.to_str().unwrap()).collect();
        assert_eq!(names, ["c.log", "a.log", "b.txt"]);
        let mut rows = [
            e("a.txt", EntryKind::File, 10),
            e("b.log", EntryKind::File, 10),
        ];
        rows.sort_by(|a, b| chained.compare(a, b));
        let names: Vec<&str> = rows.iter().map(|e| e.name.to_str().unwrap()).collect();
        assert_eq!(
            names,
            ["b.log", "a.txt"],
            "the extension broke the tie, not the name"
        );

        // The key the chain repeats is skipped, and `reverse` still turns the whole thing
        // round — the chain breaks ties, it does not escape the direction.
        let backwards = Sort {
            reverse: true,
            chain: [Some(SortKey::Size), Some(SortKey::Ext), None],
            ..by_size
        };
        rows.sort_by(|a, b| backwards.compare(a, b));
        let names: Vec<&str> = rows.iter().map(|e| e.name.to_str().unwrap()).collect();
        assert_eq!(names, ["a.txt", "b.log"]);
    }

    #[test]
    fn reversing_the_sort_turns_each_band_round_and_leaves_the_bands_where_they_are() {
        let groups = SortGroups::new(vec![band("sources", "*.rs", false)]);
        let sort = Sort {
            groups: true,
            reverse: true,
            ..Sort::default()
        };
        let mut rows = [
            e("a.rs", EntryKind::File, 0),
            e("b.rs", EntryKind::File, 0),
            e("a.txt", EntryKind::File, 0),
            e("b.txt", EntryKind::File, 0),
        ];
        rows.sort_by(|a, b| sort.compare_in(a, b, &groups));
        let names: Vec<&str> = rows.iter().map(|e| e.name.to_str().unwrap()).collect();
        // The band is still in front; what `reverse` turned round is the order inside it.
        assert_eq!(names, ["b.rs", "a.rs", "b.txt", "a.txt"]);
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

    /// A junction to a folder is a folder as far as the eye is concerned.
    #[test]
    fn a_link_that_points_at_a_directory_sorts_with_the_directories() {
        let link = Entry {
            flags: EntryFlags::LINK_DIR,
            ..e("Documents and Settings", EntryKind::Symlink, 0)
        };
        let to_a_file = e("shortcut", EntryKind::Symlink, 12);
        let rows = vec![
            e("readme.txt", EntryKind::File, 10),
            link.clone(),
            e("inetpub", EntryKind::Dir, 0),
            to_a_file.clone(),
        ];
        let sort = Sort::default();
        assert_eq!(
            names(rows, sort),
            [
                "Documents and Settings",
                "inetpub",
                "readme.txt",
                "shortcut"
            ],
            "a junction belongs with the folders, and a link to a file with the files"
        );

        // And what it *is* has not changed: it is still a link, which is what decides what
        // deleting it does.
        assert_eq!(link.kind, EntryKind::Symlink);
        assert!(link.opens_as_dir());
        assert!(!to_a_file.opens_as_dir());
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
