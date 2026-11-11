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
    /// Where the file stands in its git repository, from the same worker the `git` column
    /// asks: the changed and the new before the clean (0.39.0).
    Git,
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
    pub git: &'a str,
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
            SortKey::Git => "git",
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
        matches!(self.key, SortKey::Descr | SortKey::Owner | SortKey::Git)
            || self.asks(SortKey::Descr)
            || self.asks(SortKey::Owner)
            || self.asks(SortKey::Git)
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
            SortKey::Git => self.cmp_side(sa.git, sb.git),
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
        collate_bytes(
            a.name.as_encoded_bytes(),
            b.name.as_encoded_bytes(),
            self.natural,
            self.case_sensitive,
        )
    }

    fn cmp_str(&self, a: &str, b: &str) -> Ordering {
        collate_bytes(
            a.as_bytes(),
            b.as_bytes(),
            self.natural,
            self.case_sensitive,
        )
    }
}

/// Compare treating digit runs as numbers. `img2` < `img10`, `a` == `A` when insensitive.
///
/// [`collate_bytes`] with the digits counted, which is what everything outside a panel's own
/// sort wants of it.
pub fn natural_cmp(a: &str, b: &str, case_sensitive: bool) -> Ordering {
    collate_bytes(a.as_bytes(), b.as_bytes(), true, case_sensitive)
}

/// [`natural_cmp`] on the bytes of a name, which is how a listing holds it.
pub fn natural_cmp_bytes(a: &[u8], b: &[u8], case_sensitive: bool) -> Ordering {
    collate_bytes(a, b, true, case_sensitive)
}

/// The order of two names, on their encoded bytes: the one comparison every text key uses.
///
/// A file name is not always a `str`: on Unix it is any bytes without a slash or a zero, on
/// Windows it is UTF-16 that need not be well formed. So nothing here validates a name to
/// sort it, and a byte that is not text is never turned into a replacement character, where
/// two different names would become the same one: it sorts as the byte it is, after every
/// character (see [`Units`]).
///
/// What is compared, in this order:
///
/// 1. the name as a reader takes it in: the Latin accents folded away (`Éxtra` beside
///    `extra`, `ß` as `ss`, `æ` as `ae` — [`fold_latin`]), the case folded too unless
///    `case_sensitive`, and with `natural` a run of digits as the number it spells;
/// 2. the padding of those numbers, so `img2` and `img02` are not the same name;
/// 3. the names as they are written, which puts the accented one after the plain one — the
///    bytes when the case counts, the characters without their case when it does not.
///
/// Each step is a key of one name alone, compared like a word in a dictionary, which is what
/// makes the whole an order a sort can trust: never `a < b < c < a`. Two names come out equal
/// only when they are the same name, or differ in nothing but case with the case not counting.
///
/// This is the Latin accents and not a locale: Swedish wants `ö` after `z` and German wants
/// it with `o`, and only a collator with CLDR's tables behind it can say which. Greek and
/// Cyrillic keep the order Unicode gives them.
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
/// can share, so it walks back to the byte that begins it. From there the comparison proceeds
/// as if it had started at the first byte.
pub fn collate_bytes(a: &[u8], b: &[u8], natural: bool, case_sensitive: bool) -> Ordering {
    let common = a.len().min(b.len());
    let mut i = 0;
    while i < common && a[i] == b[i] {
        i += 1;
    }
    if i == a.len() && i == b.len() {
        return Ordering::Equal;
    }
    // One name being the other with more after it is not an answer yet: what is after may be
    // a combining accent, which the first step does not count.
    let continues = |i: usize| {
        a.get(i).is_some_and(|c| c & 0xC0 == 0x80) || b.get(i).is_some_and(|c| c & 0xC0 == 0x80)
    };
    loop {
        let before = i;
        while i > 0 && a[i - 1].is_ascii_digit() {
            i -= 1;
        }
        // A continuation byte cannot begin a character. Stepping back lands on bytes the two
        // names share, so the byte that does begin one begins it in both.
        while i > 0 && continues(i) {
            i -= 1;
        }
        // Once more round only when the character stepped over had digits before it: a
        // combining accent between two digits does not end the number.
        if i == before {
            break;
        }
    }
    collate_from(&a[i..], &b[i..], natural, case_sensitive)
}

/// The comparison itself, on the part of the two names that can still decide it.
///
/// ASCII is settled here a byte at a time, which is nearly every comparison there is. The
/// first byte above ASCII hands what is left to [`collate_tail`], the only path that decodes.
fn collate_from(a: &[u8], b: &[u8], natural: bool, case_sensitive: bool) -> Ordering {
    // `img02` and `img2` are two different files and must not compare equal, or the order
    // between them is whatever the sort happened to do. Numerically those runs are the same,
    // so the padding decides — but last, after everything else has had its say, which is what
    // keeps `a2b` before `a02c`.
    let mut padding = Ordering::Equal;
    let (mut x, mut y) = (0usize, 0usize);
    loop {
        let (ca, cb) = (a.get(x).copied(), b.get(y).copied());
        if ca.is_some_and(|c| !c.is_ascii()) || cb.is_some_and(|c| !c.is_ascii()) {
            return collate_tail(&a[x..], &b[y..], natural, case_sensitive, padding);
        }
        match (ca, cb) {
            (None, None) => return padding,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(ca), Some(cb)) => {
                if natural && ca.is_ascii_digit() && cb.is_ascii_digit() {
                    let (mut ex, mut ey) = (x, y);
                    let (na, la) = take_number(a, &mut ex);
                    let (nb, lb) = take_number(b, &mut ey);
                    // A combining accent right after the digits is not counted, so the run
                    // may go on beyond it (`1`, an accent, `2` reads as 12): where a run ends
                    // is then a question about characters, and the other path's to answer.
                    if a.get(ex).is_some_and(|c| !c.is_ascii())
                        || b.get(ey).is_some_and(|c| !c.is_ascii())
                    {
                        return collate_tail(&a[x..], &b[y..], natural, case_sensitive, padding);
                    }
                    (x, y) = (ex, ey);
                    match na.cmp(&nb) {
                        Ordering::Equal => {
                            if padding == Ordering::Equal {
                                padding = la.cmp(&lb);
                            }
                        }
                        other => return other,
                    }
                } else {
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
                }
            }
        }
    }
}

/// What is left of a comparison once one of the two names has left ASCII behind.
///
/// Same rules as the byte loop, on what [`Units`] makes of the names: runs of digits are
/// still numbers, and padding still decides before the spelling does. Both slices start
/// where a character starts: every byte before them was ASCII, or equal in the two names.
fn collate_tail(
    a: &[u8],
    b: &[u8],
    natural: bool,
    case_sensitive: bool,
    mut padding: Ordering,
) -> Ordering {
    let mut ai = Units::folded(a, !case_sensitive).peekable();
    let mut bi = Units::folded(b, !case_sensitive).peekable();
    loop {
        match (ai.peek().copied(), bi.peek().copied()) {
            (None, None) => break,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(ua), Some(ub)) => {
                if natural && is_digit(ua) && is_digit(ub) {
                    let (na, la) = take_number_units(&mut ai);
                    let (nb, lb) = take_number_units(&mut bi);
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
                    match ua.cmp(&ub) {
                        Ordering::Equal => {}
                        other => return other,
                    }
                }
            }
        }
    }
    // The two read the same. What tells them apart now is how they are written, and an
    // accented letter is a larger character than the plain one, so it comes after it.
    padding.then_with(|| {
        if case_sensitive {
            a.cmp(b)
        } else {
            Units::written(a).cmp(Units::written(b))
        }
    })
}

/// A name, one comparable step at a time: a character's code point, or — for a byte that is
/// part of no character — a number above every code point, one per byte value.
///
/// That second half is what keeps a name that is not UTF-8 itself. Decoding it "lossily"
/// would turn every stray byte into U+FFFD and sort `a\xFE` and `a\xFF` as one name; here
/// each is its own step, in byte order, after all text.
struct Units<'a> {
    bytes: &'a [u8],
    at: usize,
    /// The second letter of a character that folds to two (`ß`, `æ`), not handed out yet.
    held: Option<u32>,
    /// Fold the Latin accents and drop the combining ones: how a name reads, as opposed to
    /// how it is written.
    fold: bool,
    lower: bool,
}

/// Above every `char`: where the bytes that are not text start.
const NOT_TEXT: u32 = 0x11_0000;

impl<'a> Units<'a> {
    /// The name as it reads: accents folded, and case too when `lower`.
    fn folded(bytes: &'a [u8], lower: bool) -> Self {
        Units {
            bytes,
            at: 0,
            held: None,
            fold: true,
            lower,
        }
    }

    /// The name as it is written, without its case: what tells `é` from `e` once the two
    /// names have read the same.
    fn written(bytes: &'a [u8]) -> Self {
        Units {
            bytes,
            at: 0,
            held: None,
            fold: false,
            lower: true,
        }
    }

    /// The character that starts at the cursor, or the one byte there when it starts none.
    fn decode(&mut self) -> Option<Result<char, u8>> {
        let first = *self.bytes.get(self.at)?;
        let len = match first {
            0x00..=0x7F => 1,
            0xC2..=0xDF => 2,
            0xE0..=0xEF => 3,
            0xF0..=0xF4 => 4,
            _ => 0,
        };
        // `from_utf8` is the judge of what a character is: an overlong form or a surrogate
        // is bytes to it, as it is to the rest of the program.
        let decoded = self
            .bytes
            .get(self.at..self.at + len)
            .filter(|_| len > 0)
            .and_then(|slice| std::str::from_utf8(slice).ok())
            .and_then(|text| text.chars().next());
        Some(match decoded {
            Some(c) => {
                self.at += len;
                Ok(c)
            }
            None => {
                self.at += 1;
                Err(first)
            }
        })
    }

    fn case(&self, c: char) -> u32 {
        if !self.lower {
            c as u32
        } else if c.is_ascii() {
            c.to_ascii_lowercase() as u32
        } else {
            c.to_lowercase().next().unwrap_or(c) as u32
        }
    }
}

impl Iterator for Units<'_> {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if let Some(held) = self.held.take() {
            return Some(held);
        }
        loop {
            let c = match self.decode()? {
                Ok(c) => c,
                Err(byte) => return Some(NOT_TEXT + byte as u32),
            };
            if !self.fold {
                return Some(self.case(c));
            }
            // A combining accent (U+0300 to U+036F) is the other way of writing `é`: `e` and
            // then the accent, which is what macOS hands back for a name typed in the
            // Finder. Read, it is part of the letter before it.
            if ('\u{300}'..='\u{36F}').contains(&c) {
                continue;
            }
            return Some(match fold_latin(c) {
                Some((first, second)) => {
                    self.held = second.map(|s| self.case(s));
                    self.case(first)
                }
                None => self.case(c),
            });
        }
    }
}

/// The letters a Latin letter with an accent is read as: `é` as `e`, `Ł` as `L`, `ß` as
/// `ss`, `Æ` as `AE`. `None` for everything this does not know, which is everything outside
/// Latin-1 Supplement and Latin Extended-A, and `×` and `÷` inside them.
///
/// The case is kept, so a case-sensitive sort still puts `Éa` with the capitals.
pub fn fold_latin(c: char) -> Option<(char, Option<char>)> {
    // U+00C0 to U+017F, two bytes each: the letter, and a second one or a space. The same
    // letters as the Unicode decompositions where a letter has one; the ones that have none
    // (`ø`, `ł`, `đ`, `ß`, `æ`, `þ`…) are what a reader of the language would type instead.
    const TABLE: &[u8; 384] = b"\
A A A A A A AEC E E E E I I I I D N O O O O O   O U U U U Y THss\
a a a a a a aec e e e e i i i i d n o o o o o   o u u u u y thy \
A a A a A a C c C c C c C c D d D d E e E e E e E e E e G g G g \
G g G g H h H h I i I i I i I i I i IJijJ j K k k L l L l L l L \
l L l N n N n N n n N n O o O o O o OEoeR r R r R r S s S s S s \
S s T t T t T t U u U u U u U u U u U u W w Y y Y Z z Z z Z z s ";
    let at = (c as u32).checked_sub(0xC0)? as usize * 2;
    let first = *TABLE.get(at)?;
    let second = TABLE[at + 1];
    if first == b' ' {
        return None;
    }
    Some((first as char, (second != b' ').then_some(second as char)))
}

fn is_digit(unit: u32) -> bool {
    (b'0' as u32..=b'9' as u32).contains(&unit)
}

/// [`take_number`] over [`Units`], for the path that decodes.
fn take_number_units(it: &mut std::iter::Peekable<Units<'_>>) -> (u128, usize) {
    let mut n: u128 = 0;
    let mut digits = 0usize;
    while let Some(unit) = it.peek().copied() {
        if !is_digit(unit) {
            break;
        }
        it.next();
        digits += 1;
        n = n
            .saturating_mul(10)
            .saturating_add((unit - b'0' as u32) as u128);
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
                    let slow = collate_from(a.as_bytes(), b.as_bytes(), true, case_sensitive);
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
    fn sorted(names: &[&str], natural: bool, case_sensitive: bool) -> Vec<String> {
        let mut v: Vec<&str> = names.to_vec();
        v.sort_by(|a, b| collate_bytes(a.as_bytes(), b.as_bytes(), natural, case_sensitive));
        v.into_iter().map(str::to_string).collect()
    }

    #[test]
    fn an_accented_name_sorts_beside_the_plain_one_and_not_after_z() {
        // The complaint this answers: on its bytes `Éxtra.txt` came after `zorro.txt`.
        for case_sensitive in [true, false] {
            assert_eq!(
                natural_cmp("Éxtra.txt", "zorro.txt", case_sensitive),
                Ordering::Less
            );
            assert_eq!(
                natural_cmp("éxtra.txt", "fine.txt", case_sensitive),
                Ordering::Less
            );
        }
        assert_eq!(
            sorted(
                &["zorro", "étude", "eta", "Łódź", "lodz", "mañana", "manzana", "eve"],
                true,
                false
            ),
            ["eta", "étude", "eve", "lodz", "Łódź", "mañana", "manzana", "zorro"]
        );
    }

    #[test]
    fn two_names_that_read_the_same_put_the_plain_one_first() {
        for case_sensitive in [true, false] {
            assert_eq!(
                natural_cmp("extra", "éxtra", case_sensitive),
                Ordering::Less
            );
            assert_eq!(
                natural_cmp("éxtra", "extra", case_sensitive),
                Ordering::Greater
            );
            // And the accent decides only after the whole name has been read.
            assert_eq!(
                natural_cmp("éxtra1", "extra2", case_sensitive),
                Ordering::Less
            );
        }
    }

    #[test]
    fn a_letter_that_is_two_is_read_as_two() {
        for case_sensitive in [true, false] {
            // `ß` is `ss`: between `strasbourg` and `strast`, not after `z`.
            assert_eq!(
                sorted(
                    &["straße", "strast", "strasse", "strasbourg", "strasz"],
                    true,
                    case_sensitive
                ),
                ["strasbourg", "strasse", "straße", "strast", "strasz"]
            );
            assert_eq!(natural_cmp("æon", "afar", case_sensitive), Ordering::Less);
            assert_eq!(
                natural_cmp("æon", "aeon", case_sensitive),
                Ordering::Greater
            );
            assert_eq!(natural_cmp("æon", "aez", case_sensitive), Ordering::Less);
            assert_eq!(natural_cmp("œuvre", "of", case_sensitive), Ordering::Less);
            assert_eq!(
                natural_cmp("søren", "sorry", case_sensitive),
                Ordering::Less
            );
            assert_eq!(natural_cmp("łuk", "lux", case_sensitive), Ordering::Less);
        }
        assert_eq!(fold_latin('ß'), Some(('s', Some('s'))));
        assert_eq!(fold_latin('Æ'), Some(('A', Some('E'))));
        assert_eq!(fold_latin('ø'), Some(('o', None)));
        assert_eq!(fold_latin('Ł'), Some(('L', None)));
        assert_eq!(fold_latin('ı'), Some(('i', None)));
        assert_eq!(fold_latin('ž'), Some(('z', None)));
        assert_eq!(fold_latin('ſ'), Some(('s', None)));
        // The two signs in the middle of the letters, and everything on either side.
        assert_eq!(fold_latin('×'), None);
        assert_eq!(fold_latin('÷'), None);
        assert_eq!(fold_latin('¿'), None);
        assert_eq!(fold_latin('ƀ'), None);
        assert_eq!(fold_latin('e'), None);
        assert_eq!(fold_latin('я'), None);
    }

    #[test]
    fn an_accent_written_after_its_letter_reads_like_the_one_written_in_it() {
        // macOS hands names back decomposed: `e` and U+0301 where Linux has U+00E9.
        let decomposed = "cafe\u{301}";
        for case_sensitive in [true, false] {
            assert_eq!(
                natural_cmp(decomposed, "cafz", case_sensitive),
                Ordering::Less
            );
            assert_eq!(
                natural_cmp(decomposed, "cafe", case_sensitive),
                Ordering::Greater
            );
            assert_eq!(
                natural_cmp("cafe", decomposed, case_sensitive),
                Ordering::Less
            );
            assert_ne!(
                natural_cmp(decomposed, "café", case_sensitive),
                Ordering::Equal
            );
        }
    }

    #[test]
    fn the_case_counts_only_when_asked_and_the_accents_fold_either_way() {
        // Sensitive: capitals first, as the bytes always had it, and `É` with the capitals.
        assert_eq!(
            sorted(&["extra", "Zeta", "Éxtra", "alpha", "Extra"], true, true),
            ["Extra", "Éxtra", "Zeta", "alpha", "extra"]
        );
        // Insensitive: one alphabet.
        assert_eq!(
            sorted(&["zeta", "Éxtra", "alpha", "Beta"], true, false),
            ["alpha", "Beta", "Éxtra", "zeta"]
        );
        assert_eq!(natural_cmp("ÉCOLE", "école", false), Ordering::Equal);
        assert_ne!(natural_cmp("ÉCOLE", "école", true), Ordering::Equal);
        assert_eq!(natural_cmp("Ecole", "école", false), Ordering::Less);
    }

    #[test]
    fn numbers_are_still_numbers_beside_an_accent() {
        for case_sensitive in [true, false] {
            assert_eq!(
                sorted(
                    &["été10", "ete2", "été2", "ete10", "été02"],
                    true,
                    case_sensitive
                ),
                ["ete2", "été2", "été02", "ete10", "été10"]
            );
            // And with the natural order off a digit is a character like any other, the
            // accents folded all the same.
            assert_eq!(
                sorted(&["été10", "ete2", "été2", "zz"], false, case_sensitive),
                ["été10", "ete2", "été2", "zz"]
            );
        }
        assert_eq!(
            collate_bytes(b"img2", b"img10", false, true),
            Ordering::Greater
        );
    }

    #[test]
    fn a_name_that_is_not_text_keeps_every_byte_it_has() {
        for case_sensitive in [true, false] {
            for natural in [true, false] {
                let cmp = |a: &[u8], b: &[u8]| collate_bytes(a, b, natural, case_sensitive);
                // Two different stray bytes are two different names, in byte order: decoded
                // "lossily" both would be U+FFFD and the two names one.
                assert_eq!(cmp(b"a\xFE", b"a\xFF"), Ordering::Less);
                assert_eq!(cmp(b"a\xFF", b"a\xFE"), Ordering::Greater);
                assert_eq!(cmp(b"a\x80z", b"a\x81a"), Ordering::Less);
                // A byte that is no character comes after every character, as `\xFF` always
                // did, and is not folded into anything.
                assert_eq!(cmp(b"a\xE9", "aé".as_bytes()), Ordering::Greater);
                assert_eq!(cmp(b"a\xE9", b"az"), Ordering::Greater);
                assert_eq!(cmp(b"a\xFF", "a日".as_bytes()), Ordering::Greater);
                // The head of a character with its tail cut off is bytes too.
                assert_eq!(cmp(b"a\xC3", "aé".as_bytes()), Ordering::Greater);
                assert_eq!(cmp(b"a\xC3", b"a\xC3"), Ordering::Equal);
            }
        }
    }

    /// The property a sort needs and a comparison assembled from special cases loses first:
    /// one answer per pair, the opposite answer the other way round, and no cycle.
    #[test]
    fn the_order_is_total_on_names_of_every_kind() {
        let names: Vec<&[u8]> = vec![
            b"",
            b"a",
            b"A",
            b"e",
            b"E",
            "é".as_bytes(),
            "É".as_bytes(),
            "e\u{301}".as_bytes(),
            "ë".as_bytes(),
            b"ea",
            "éa".as_bytes(),
            b"ez",
            b"f",
            b"z",
            b"ss",
            b"sS",
            "ß".as_bytes(),
            b"st",
            b"sr",
            b"ae",
            "æ".as_bytes(),
            "Æ".as_bytes(),
            b"af",
            b"1",
            b"01",
            b"2",
            b"10",
            "1\u{301}2".as_bytes(),
            "1\u{300}03".as_bytes(),
            b"12",
            b"103",
            "é2".as_bytes(),
            "é10".as_bytes(),
            "é02".as_bytes(),
            b"e2",
            b"e10",
            b"\xC3",
            b"\xC3\x28",
            b"\xA9",
            b"\xFF",
            b"e\xFF",
            b"e\xFE",
            b"z\x80",
            "日本".as_bytes(),
            "я".as_bytes(),
            "Я".as_bytes(),
            "×".as_bytes(),
            b"~",
        ];
        for natural in [true, false] {
            for case_sensitive in [true, false] {
                let cmp = |a: &[u8], b: &[u8]| collate_bytes(a, b, natural, case_sensitive);
                let what = format!("natural = {natural}, case_sensitive = {case_sensitive}");
                for a in &names {
                    assert_eq!(cmp(a, a), Ordering::Equal, "{a:?} with itself ({what})");
                    for b in &names {
                        let ab = cmp(a, b);
                        assert_eq!(ab, cmp(b, a).reverse(), "{a:?} and {b:?} ({what})");
                        // The skip over the shared head must not change the answer.
                        assert_eq!(
                            ab,
                            collate_from(a, b, natural, case_sensitive),
                            "{a:?} and {b:?}, the long way ({what})"
                        );
                        if case_sensitive && a != b {
                            assert_ne!(ab, Ordering::Equal, "{a:?} and {b:?} ({what})");
                        }
                        for c in &names {
                            if ab != Ordering::Greater && cmp(b, c) != Ordering::Greater {
                                assert_ne!(
                                    cmp(a, c),
                                    Ordering::Greater,
                                    "{a:?} <= {b:?} <= {c:?} and yet {a:?} > {c:?} ({what})"
                                );
                            }
                        }
                    }
                }
                // And sorting the same names from two different shuffles ends the same way.
                let mut one = names.clone();
                let mut two: Vec<&[u8]> = names.iter().rev().copied().collect();
                one.sort_by(|a, b| cmp(a, b).then_with(|| a.cmp(b)));
                two.sort_by(|a, b| cmp(a, b).then_with(|| a.cmp(b)));
                assert_eq!(one, two, "{what}");
            }
        }
    }

    #[test]
    fn a_panel_sorted_by_name_puts_the_accented_files_where_they_are_looked_for() {
        let listing = vec![
            e("zorro.txt", EntryKind::File, 0),
            e("Éxtra.txt", EntryKind::File, 0),
            e("extra.txt", EntryKind::File, 0),
            e("fine.txt", EntryKind::File, 0),
        ];
        let sort = Sort {
            case_sensitive: false,
            ..Sort::default()
        };
        assert_eq!(
            names(listing.clone(), sort),
            ["extra.txt", "Éxtra.txt", "fine.txt", "zorro.txt"]
        );
        // The same with the natural order off: the folding is not part of that switch.
        let plain = Sort {
            natural: false,
            ..sort
        };
        assert_eq!(
            names(listing, plain),
            ["extra.txt", "Éxtra.txt", "fine.txt", "zorro.txt"]
        );
    }

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
