use crate::entry::{Entry, EntryFlags, EntryId, EntryKind};
use crate::selection::Selection;
use crate::sort::Sort;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Totals {
    pub files: u64,
    pub dirs: u64,
    pub bytes: u64,
    pub selected_files: u64,
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
}

impl Listing {
    pub fn new(entries: Vec<Entry>, generation: u64) -> Self {
        let mut l = Listing {
            entries,
            order: Vec::new(),
            totals: Totals::default(),
            generation,
            truncated: false,
        };
        l.rebuild(Sort::default(), false, None, &Selection::default());
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
    /// `quick_filter` is a lowercase substring; `None` means no filter.
    pub fn rebuild(
        &mut self,
        sort: Sort,
        show_hidden: bool,
        quick_filter: Option<&str>,
        selection: &Selection,
    ) {
        self.order.clear();
        self.order.reserve(self.entries.len());
        let mut totals = Totals::default();

        for (i, e) in self.entries.iter().enumerate() {
            if e.kind == EntryKind::Parent {
                self.order.push(i as u32);
                continue;
            }
            if !show_hidden && e.flags.contains(EntryFlags::HIDDEN) {
                continue;
            }
            if let Some(needle) = quick_filter {
                if !e.name_str().to_lowercase().contains(needle) {
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
                totals.selected_files += 1;
                totals.selected_bytes += e.size;
            }
            self.order.push(i as u32);
        }

        let entries = &self.entries;
        if sort.selected_first {
            self.order.sort_by(|a, b| {
                let (ea, eb) = (&entries[*a as usize], &entries[*b as usize]);
                let sa = selection.contains(ea.id);
                let sb = selection.contains(eb.id);
                sb.cmp(&sa).then_with(|| sort.compare(ea, eb))
            });
        } else {
            self.order
                .sort_by(|a, b| sort.compare(&entries[*a as usize], &entries[*b as usize]));
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
        l.rebuild(Sort::default(), true, None, &Selection::default());
        assert_eq!(l.len(), 5);
        assert_eq!(l.totals.files, 3);
    }

    #[test]
    fn quick_filter_keeps_the_parent_entry() {
        let mut l = sample();
        l.rebuild(Sort::default(), false, Some("a."), &Selection::default());
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
        l.rebuild(sort, false, None, &sel);
        let names: Vec<_> = (0..l.len())
            .map(|i| l.row(i).unwrap().name_str().into_owned())
            .collect();
        assert_eq!(names[0], "b.txt");
        assert_eq!(l.totals.selected_bytes, 20);
    }
}
