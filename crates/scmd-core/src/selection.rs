use std::collections::HashSet;

use globset::{Glob, GlobSetBuilder};

use crate::entry::{Entry, EntryId, EntryKind};
use crate::listing::Listing;

/// What a mask operation applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaskScope {
    FilesOnly,
    FilesAndDirs,
}

/// The set of selected entries, by stable id, so it survives re-sorting and re-reading.
#[derive(Debug, Clone, Default)]
pub struct Selection {
    ids: HashSet<EntryId>,
    previous: Option<HashSet<EntryId>>,
}

impl Selection {
    pub fn contains(&self, id: EntryId) -> bool {
        self.ids.contains(&id)
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    pub fn toggle(&mut self, id: EntryId) {
        if !self.ids.remove(&id) {
            self.ids.insert(id);
        }
    }

    pub fn set(&mut self, id: EntryId, on: bool) {
        if on {
            self.ids.insert(id);
        } else {
            self.ids.remove(&id);
        }
    }

    pub fn clear(&mut self) {
        self.remember();
        self.ids.clear();
    }

    /// `Ctrl+M` in FAR: bring back the selection as it was before the last bulk change.
    pub fn restore_previous(&mut self) {
        if let Some(prev) = self.previous.take() {
            self.previous = Some(self.ids.clone());
            self.ids = prev;
        }
    }

    fn remember(&mut self) {
        self.previous = Some(self.ids.clone());
    }

    pub fn select_all(&mut self, listing: &Listing, scope: MaskScope) {
        self.remember();
        for row in 0..listing.len() {
            let e = listing.row(row).expect("row in range");
            if !e.is_selectable() {
                continue;
            }
            if scope == MaskScope::FilesOnly && e.kind == EntryKind::Dir {
                continue;
            }
            self.ids.insert(e.id);
        }
    }

    pub fn invert(&mut self, listing: &Listing, scope: MaskScope) {
        self.remember();
        for row in 0..listing.len() {
            let e = listing.row(row).expect("row in range");
            if !e.is_selectable() {
                continue;
            }
            if scope == MaskScope::FilesOnly && e.kind == EntryKind::Dir {
                continue;
            }
            if !self.ids.remove(&e.id) {
                self.ids.insert(e.id);
            }
        }
    }

    /// Apply a comma-separated mask list (`*.rs,*.toml`) to the visible rows.
    ///
    /// Returns the number of entries whose state changed. An invalid pattern is an error, not
    /// a silent no-op — the UI shows it.
    pub fn apply_mask(
        &mut self,
        listing: &Listing,
        masks: &str,
        on: bool,
        scope: MaskScope,
    ) -> Result<usize, globset::Error> {
        let mut builder = GlobSetBuilder::new();
        for raw in masks.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            let pattern = if raw.contains('*') || raw.contains('?') || raw.contains('[') {
                raw.to_string()
            } else {
                // A bare word means "contains", like the commanders do.
                format!("*{raw}*")
            };
            builder.add(Glob::new(&pattern).or_else(|_| Glob::new(&globset::escape(raw)))?);
        }
        let set = builder.build()?;

        self.remember();
        let mut changed = 0;
        for row in 0..listing.len() {
            let e = listing.row(row).expect("row in range");
            if !e.is_selectable() {
                continue;
            }
            if scope == MaskScope::FilesOnly && e.kind == EntryKind::Dir {
                continue;
            }
            let name = e.name_str();
            let hit = set.is_match(name.as_ref()) || set.is_match(name.to_lowercase().as_str());
            if hit && self.contains(e.id) != on {
                self.set(e.id, on);
                changed += 1;
            }
        }
        Ok(changed)
    }

    /// Select or deselect exactly the rows a predicate picks, remembering what was selected
    /// before so that `restore_previous` undoes the whole sweep rather than one row. FAR's
    /// modifiers on the grey keys all come down to this: the criterion changes, the mechanics
    /// do not. Returns how many rows changed state.
    pub fn apply_where(
        &mut self,
        listing: &Listing,
        scope: MaskScope,
        on: bool,
        mut pick: impl FnMut(&Entry) -> bool,
    ) -> usize {
        self.remember();
        let mut changed = 0;
        for row in 0..listing.len() {
            let e = listing.row(row).expect("row in range");
            if !e.is_selectable() || (scope == MaskScope::FilesOnly && e.kind == EntryKind::Dir) {
                continue;
            }
            if pick(e) && self.contains(e.id) != on {
                self.set(e.id, on);
                changed += 1;
            }
        }
        changed
    }

    /// Drop ids that are no longer present, after a re-read.
    pub fn retain_existing(&mut self, listing: &Listing) {
        let present: HashSet<EntryId> = listing.entries.iter().map(|e| e.id).collect();
        self.ids.retain(|id| present.contains(id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::{Entry, EntryFlags};
    use std::ffi::OsString;

    fn listing() -> Listing {
        let mk = |name: &str, kind: EntryKind| Entry {
            name: OsString::from(name),
            kind,
            size: 1,
            mtime: None,
            flags: EntryFlags::empty(),
            id: EntryId::Name(name.bytes().map(u64::from).sum()),
            link_target: None,
        };
        Listing::new(
            vec![
                Entry::parent_entry(),
                mk("main.rs", EntryKind::File),
                mk("lib.rs", EntryKind::File),
                mk("Cargo.toml", EntryKind::File),
                mk("src", EntryKind::Dir),
            ],
            1,
        )
    }

    #[test]
    fn mask_selects_matching_files_only() {
        let l = listing();
        let mut s = Selection::default();
        let n = s
            .apply_mask(&l, "*.rs", true, MaskScope::FilesOnly)
            .unwrap();
        assert_eq!(n, 2);
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn bare_word_mask_means_contains() {
        let l = listing();
        let mut s = Selection::default();
        s.apply_mask(&l, "cargo", true, MaskScope::FilesOnly)
            .unwrap();
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn parent_entry_is_never_selected() {
        let l = listing();
        let mut s = Selection::default();
        s.select_all(&l, MaskScope::FilesAndDirs);
        assert_eq!(s.len(), 4);
        let parent = l.row(0).unwrap();
        assert!(!s.contains(parent.id));
    }

    #[test]
    fn invert_and_restore() {
        let l = listing();
        let mut s = Selection::default();
        s.apply_mask(&l, "*.rs", true, MaskScope::FilesOnly)
            .unwrap();
        s.invert(&l, MaskScope::FilesAndDirs);
        assert_eq!(s.len(), 2); // Cargo.toml + src
        s.restore_previous();
        assert_eq!(s.len(), 2); // back to the two .rs files
        assert!(s.contains(
            l.row_of_name("main.rs")
                .map(|r| l.row(r).unwrap().id)
                .unwrap()
        ));
    }

    #[test]
    fn a_predicate_selects_a_group_and_one_restore_undoes_it() {
        let l = listing();
        let mut s = Selection::default();
        // Everything with the same extension as a `.rs` file, the way Ctrl+Gray+ works.
        let picked = s.apply_where(&l, MaskScope::FilesOnly, true, |e| {
            std::path::Path::new(&e.name)
                .extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("rs"))
        });
        assert!(picked > 0, "nothing matched");
        assert_eq!(s.len(), picked);
        // One restore puts the whole sweep back, not the last row of it.
        s.restore_previous();
        assert!(s.is_empty(), "restore did not undo the sweep");
    }
}
