use crate::entry::{Entry, EntryFlags, EntryId, EntryKind};
use crate::selection::Selection;
use crate::sort::{Sort, SortGroups};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Totals {
    pub files: u64,
    pub dirs: u64,
    pub bytes: u64,
    /// Everything selected, directories included — what "N selected" means to the user.
    pub selected_files: u64,
    /// How many of those are directories, for the panel's selected-totals bar.
    pub selected_dirs: u64,
    pub selected_bytes: u64,
}

/// An immutable-ish snapshot of a directory plus the sorted/filtered view onto it.
///
/// `entries` is the raw read; `order` is what the panel shows. Re-sorting or re-filtering
/// rebuilds `order` only — entries are never moved, so indices stored elsewhere stay valid.
#[derive(Debug, Clone, Default)]
pub struct Listing {
    pub entries: Vec<Entry>,
    pub order: Vec<u32>,
    pub totals: Totals,
    pub generation: u64,
    pub truncated: bool,
    /// How long the longest visible name is, in bytes of its own encoding.
    ///
    /// Bytes and not display columns: this is worked out in the same pass that filters the
    /// listing, which for a directory of 1.7 million rows is a pass worth keeping cheap, and
    /// validating every name as UTF-8 to measure it would double what that pass costs. For an
    /// ASCII name the two are the same number; for a name with accents in it the bytes are
    /// the larger, and the one place this is used — how many columns the wide view fits —
    /// errs towards fewer columns, which is the safe direction.
    pub widest: usize,
}

/// How the two fields that live beside the listing are looked up, per entry: the description
/// and the owner, in that order. A closure rather than the maps themselves, because what
/// holds them is the panel and what sorts is the listing.
pub type SideLookup<'a> = dyn Fn(&Entry) -> (String, String) + 'a;

impl Listing {
    pub fn new(entries: Vec<Entry>, generation: u64) -> Self {
        let mut l = Listing {
            entries,
            order: Vec::new(),
            totals: Totals::default(),
            generation,
            truncated: false,
            widest: 0,
        };
        l.rebuild(
            Sort::default(),
            false,
            None,
            &Selection::default(),
            &SortGroups::default(),
        );
        l
    }

    /// Number of visible rows.
    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    /// Visible row `row` → entry.
    pub fn row(&self, row: usize) -> Option<&Entry> {
        self.order.get(row).map(|i| &self.entries[*i as usize])
    }

    pub fn row_of_id(&self, id: EntryId) -> Option<usize> {
        self.order
            .iter()
            .position(|i| self.entries[*i as usize].id == id)
    }

    pub fn row_of_name(&self, name: &str) -> Option<usize> {
        self.order
            .iter()
            .position(|i| self.entries[*i as usize].name_str() == name)
    }

    /// Recompute the visible order and the totals.
    ///
    /// `filter` is a mask set (a bare word still means "contains", so the quick filter bar
    /// behaves as it always did); `None` means no filter.
    pub fn rebuild(
        &mut self,
        sort: Sort,
        show_hidden: bool,
        filter: Option<&crate::filter::MaskSet>,
        selection: &Selection,
        groups: &crate::sort::SortGroups,
    ) {
        self.rebuild_with(sort, show_hidden, filter, selection, groups, None);
    }

    /// The same, with the two fields that live beside the listing — the description and the
    /// owner — looked up per entry for the keys that sort by them (SPEC 02 §4).
    ///
    /// `side` is asked once per entry, before the sort, rather than inside the comparison:
    /// a sort of a million rows asks the comparison some thirty-five million times, and a
    /// map lookup in there would be thirty-five million hashes of a name.
    #[allow(clippy::too_many_arguments)]
    pub fn rebuild_with(
        &mut self,
        sort: Sort,
        show_hidden: bool,
        filter: Option<&crate::filter::MaskSet>,
        selection: &Selection,
        groups: &crate::sort::SortGroups,
        side: Option<&SideLookup<'_>>,
    ) {
        self.order.clear();
        self.order.reserve(self.entries.len());
        let mut totals = Totals::default();
        // Which rows are selected, by index, answered once here instead of hashed twice per
        // comparison inside the sort. The filter pass below already asks the question for the
        // totals, so this costs one bit-ish per entry and saves some seventy million hashes
        // on a panel with a million rows in it.
        let mut selected = vec![false; self.entries.len()];
        let mut widest = 0usize;

        for (i, e) in self.entries.iter().enumerate() {
            if e.kind == EntryKind::Parent {
                self.order.push(i as u32);
                continue;
            }
            if !show_hidden && e.flags.contains(EntryFlags::HIDDEN) {
                continue;
            }
            if let Some(masks) = filter {
                if !masks.matches(e) {
                    continue;
                }
            }
            match e.kind {
                EntryKind::Dir => totals.dirs += 1,
                _ => {
                    totals.files += 1;
                    totals.bytes += e.size;
                }
            }
            if selection.contains(e.id) {
                selected[i] = true;
                totals.selected_files += 1;
                totals.selected_bytes += e.size;
                if e.kind == EntryKind::Dir {
                    totals.selected_dirs += 1;
                }
            }
            widest = widest.max(e.name.as_encoded_bytes().len());
            self.order.push(i as u32);
        }
        self.widest = widest;

        let entries = &self.entries;
        // The side fields, per entry, where a key asks for them. One pass over the rows that
        // are actually on screen, so a directory nobody is sorting by description costs
        // nothing at all.
        let sides: Vec<(String, String)> = match side.filter(|_| sort.needs_side()) {
            Some(lookup) => entries.iter().map(lookup).collect(),
            None => Vec::new(),
        };
        let side_of = |i: usize| -> crate::sort::Side<'_> {
            match sides.get(i) {
                Some((descr, owner)) => crate::sort::Side { descr, owner },
                None => crate::sort::Side::default(),
            }
        };
        // Unstable: it is the faster sort and there is nothing for stability to preserve,
        // because the comparison is a total order — two rows only compare equal when they
        // carry the same name, and a directory cannot hold two of those. Except under
        // `Unsorted`, which answers `Equal` to everything on purpose: there the order the
        // provider gave is the order, and a sort that may permute equal elements would throw
        // away the only thing being asked for, so that case is not sorted at all beyond the
        // grouping the comparison still has to do.
        // A stable sort, and measured rather than assumed: the unstable one moves less memory
        // but performs more comparisons, and a comparison here reaches into a name somewhere
        // else in the heap, so on 1.7 million rows it came out a third slower. Stability also
        // means `Unsorted` keeps the order the provider gave, which is the whole of what it
        // asks for.
        if sort.selected_first {
            self.order.sort_by(|a, b| {
                let (ia, ib) = (*a as usize, *b as usize);
                selected[ib].cmp(&selected[ia]).then_with(|| {
                    sort.compare_side(&entries[ia], &entries[ib], groups, side_of(ia), side_of(ib))
                })
            });
        } else {
            self.order.sort_by(|a, b| {
                let (ia, ib) = (*a as usize, *b as usize);
                sort.compare_side(&entries[ia], &entries[ib], groups, side_of(ia), side_of(ib))
            });
        }

        self.totals = totals;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    fn e(name: &str, kind: EntryKind, size: u64, hidden: bool) -> Entry {
        let mut flags = EntryFlags::empty();
        flags.set(EntryFlags::HIDDEN, hidden);
        Entry {
            name: OsString::from(name),
            kind,
            size,
            mtime: None,
            btime: None,
            atime: None,
            flags,
            id: EntryId::Name(name.as_bytes().iter().map(|b| *b as u64).sum()),
            link_target: None,
        }
    }

    fn sample() -> Listing {
        Listing::new(
            vec![
                Entry::parent_entry(),
                e("b.txt", EntryKind::File, 20, false),
                e(".hidden", EntryKind::File, 5, true),
                e("a.txt", EntryKind::File, 10, false),
                e("dir", EntryKind::Dir, 0, false),
            ],
            1,
        )
    }

    #[test]
    fn hidden_entries_are_filtered_but_countable() {
        let l = sample();
        let names: Vec<_> = (0..l.len())
            .map(|i| l.row(i).unwrap().name_str().into_owned())
            .collect();
        assert_eq!(names, vec!["..", "dir", "a.txt", "b.txt"]);
        assert_eq!(l.totals.files, 2);
        assert_eq!(l.totals.bytes, 30);
    }

    #[test]
    fn show_hidden_adds_them_back() {
        let mut l = sample();
        l.rebuild(
            Sort::default(),
            true,
            None,
            &Selection::default(),
            &SortGroups::default(),
        );
        assert_eq!(l.len(), 5);
        assert_eq!(l.totals.files, 3);
    }

    #[test]
    fn quick_filter_keeps_the_parent_entry() {
        let mut l = sample();
        let masks = crate::filter::MaskSet::parse("a.").unwrap();
        l.rebuild(
            Sort::default(),
            false,
            Some(&masks),
            &Selection::default(),
            &SortGroups::default(),
        );
        let names: Vec<_> = (0..l.len())
            .map(|i| l.row(i).unwrap().name_str().into_owned())
            .collect();
        assert_eq!(names, vec!["..", "a.txt"]);
    }

    #[test]
    fn selected_first_reorders_without_losing_the_parent() {
        let mut l = sample();
        let id = l
            .entries
            .iter()
            .find(|e| e.name_str() == "b.txt")
            .unwrap()
            .id;
        let mut sel = Selection::default();
        sel.toggle(id);
        let sort = Sort {
            selected_first: true,
            ..Sort::default()
        };
        l.rebuild(sort, false, None, &sel, &SortGroups::default());
        let names: Vec<_> = (0..l.len())
            .map(|i| l.row(i).unwrap().name_str().into_owned())
            .collect();
        assert_eq!(names[0], "b.txt");
        assert_eq!(l.totals.selected_bytes, 20);
    }
}

/// What a panel over a very large bucket costs, measured rather than guessed.
///
/// Not a test of behaviour and not run by `cargo test`: `cargo test --release -p fcmd-core
/// --lib a_bucket_sized_listing -- --ignored --nocapture` prints the numbers. It is here
/// because the question "could it open a bucket with 1.7 million objects in it" deserves an
/// answer with figures attached, and because the figures move when this file does.
///
/// The names are shaped like the keys such a bucket really holds —
/// `2024/08/13/customer-00012345/event-0000000678.json` — which matters: they share thirty
/// characters with their neighbours and they are full of digits, so they are the worst case
/// for a natural order and nothing like the short names a test directory has.
#[cfg(test)]
mod scale {
    use super::*;
    use crate::sort::{SortGroups, SortKey};
    use std::ffi::OsString;
    use std::time::Instant;

    #[test]
    #[ignore]
    fn a_bucket_sized_listing() {
        let n = 1_700_000usize;
        eprintln!("size_of::<Entry>() = {}", std::mem::size_of::<Entry>());
        let t = Instant::now();
        let mut entries: Vec<Entry> = Vec::with_capacity(n + 1);
        entries.push(Entry::parent_entry());
        let mut names = 0usize;
        for i in 0..n {
            let name = format!(
                "{:04}/{:02}/{:02}/customer-{:08}/event-{:010}.json",
                2020 + i % 6,
                1 + i % 12,
                1 + i % 28,
                i % 50_000,
                i
            );
            names += name.len();
            entries.push(Entry {
                name: OsString::from(name),
                kind: EntryKind::File,
                size: (i as u64 * 7919) % 4_000_000,
                mtime: Some(
                    std::time::SystemTime::UNIX_EPOCH
                        + std::time::Duration::from_secs(1_600_000_000 + i as u64),
                ),
                btime: None,
                atime: None,
                flags: EntryFlags::empty(),
                id: EntryId::Name(i as u64),
                link_target: None,
            });
        }
        eprintln!("built {} rows in {:?}", entries.len(), t.elapsed());
        eprintln!(
            "names {:.0} MiB + rows {:.0} MiB + order {:.0} MiB = {:.0} MiB",
            names as f64 / 1048576.0,
            (entries.len() * std::mem::size_of::<Entry>()) as f64 / 1048576.0,
            (entries.len() * 4) as f64 / 1048576.0,
            (names + entries.len() * (std::mem::size_of::<Entry>() + 4)) as f64 / 1048576.0
        );

        let t = Instant::now();
        let mut l = Listing::new(entries, 1);
        eprintln!("sort by name            {:?}", t.elapsed());

        let t = Instant::now();
        let masks = crate::filter::MaskSet::parse("*.json").unwrap();
        l.rebuild(
            Sort::default(),
            false,
            Some(&masks),
            &Selection::default(),
            &SortGroups::default(),
        );
        eprintln!(
            "filter *.json and sort  {:?} ({} rows)",
            t.elapsed(),
            l.len()
        );

        let t = Instant::now();
        l.rebuild(
            Sort {
                key: SortKey::Size,
                ..Sort::default()
            },
            false,
            None,
            &Selection::default(),
            &SortGroups::default(),
        );
        eprintln!("sort by size            {:?}", t.elapsed());

        let t = Instant::now();
        let row = l.row_of_name("2020/01/01/customer-00000000/event-0000000000.json");
        eprintln!("find a row by name      {:?} ({row:?})", t.elapsed());

        let mut sel = Selection::default();
        let t = Instant::now();
        for i in 0..n as u64 {
            sel.toggle(EntryId::Name(i));
        }
        eprintln!("select every row        {:?}", t.elapsed());
        let t = Instant::now();
        l.rebuild(
            Sort {
                selected_first: true,
                ..Sort::default()
            },
            false,
            None,
            &sel,
            &SortGroups::default(),
        );
        eprintln!("sort selected first     {:?}", t.elapsed());
    }
}
