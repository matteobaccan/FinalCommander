//! FAR's default file highlighting: which colour a row gets before the cursor and the
//! selection have their say.
//!
//! The groups and their colours are FAR's own defaults, group for group and in the same
//! order (`far/hilight.cpp` of FarGroup/FarManager, BSD-3-Clause). The order is the whole
//! point: the first group that matches decides, which is why hidden and system come before
//! everything else — a hidden directory is cyan in FAR, not white.

use scmd_core::filter::MaskSet;
use scmd_core::{Entry, EntryFlags, EntryKind};

/// The default groups, compiled once. Building a `MaskSet` parses and compiles its masks, so
/// this is done at startup rather than per row.
#[derive(Debug, Clone)]
pub struct Highlight {
    exec: Option<MaskSet>,
    arc: Option<MaskSet>,
    temp: Option<MaskSet>,
}

impl Default for Highlight {
    fn default() -> Self {
        Highlight::far_default()
    }
}

impl Highlight {
    /// FAR's six default groups. A group whose masks do not compile is simply absent: a theme
    /// is not worth failing a startup over.
    pub fn far_default() -> Self {
        let groups = scmd_core::filter::default_groups();
        // Case-insensitive, because `.ZIP` is an archive on every platform a user cares
        // about, and FAR compares masks without case.
        let compile = |name: &str| MaskSet::parse_with(name, &groups, false).ok();
        Highlight {
            exec: compile("<exec>"),
            arc: compile("<arc>"),
            temp: compile("<temp>"),
        }
    }

    /// Nothing is highlighted: every file takes the panel's own colour. What a theme that
    /// wants one flat colour asks for.
    pub fn none() -> Self {
        Highlight {
            exec: None,
            arc: None,
            temp: None,
        }
    }

    /// The theme key for this entry's colour, in FAR's group order.
    pub fn key(&self, e: &Entry) -> &'static str {
        // Hidden and system first, as the group order puts them: in FAR a hidden directory
        // is cyan, not white.
        if e.flags.contains(EntryFlags::BROKEN_LINK) {
            return "file.broken";
        }
        if e.flags.contains(EntryFlags::HIDDEN) || e.flags.contains(EntryFlags::SYSTEM) {
            return "file.hidden";
        }
        if matches!(e.kind, EntryKind::Dir | EntryKind::Parent) {
            return "file.dir";
        }
        if e.kind == EntryKind::Symlink {
            return "file.symlink";
        }
        // The executable *bit* counts as the executable group too: on Unix that is what makes
        // a file runnable, where FAR has only the name to go by.
        if e.flags.contains(EntryFlags::EXECUTABLE) || self.matches(&self.exec, e) {
            return "file.exec";
        }
        if self.matches(&self.arc, e) {
            return "file.archive";
        }
        if self.matches(&self.temp, e) {
            return "file.temp";
        }
        "panel.fg"
    }

    fn matches(&self, set: &Option<MaskSet>, e: &Entry) -> bool {
        set.as_ref().is_some_and(|s| s.matches(e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scmd_core::{EntryId, EntryKind};
    use std::ffi::OsString;

    fn entry(name: &str, kind: EntryKind, flags: EntryFlags) -> Entry {
        Entry {
            name: OsString::from(name),
            kind,
            size: 0,
            mtime: None,
            btime: None,
            atime: None,
            flags,
            id: EntryId::Name(1),
            link_target: None,
        }
    }

    #[test]
    fn the_groups_decide_in_fars_own_order() {
        let h = Highlight::far_default();
        let plain = EntryFlags::empty();
        assert_eq!(
            h.key(&entry("notes.txt", EntryKind::File, plain)),
            "panel.fg"
        );
        assert_eq!(h.key(&entry("src", EntryKind::Dir, plain)), "file.dir");
        assert_eq!(h.key(&entry("run.sh", EntryKind::File, plain)), "file.exec");
        assert_eq!(
            h.key(&entry("src.zip", EntryKind::File, plain)),
            "file.archive"
        );
        assert_eq!(
            h.key(&entry("notes.bak", EntryKind::File, plain)),
            "file.temp"
        );
        assert_eq!(h.key(&entry("vi.swp", EntryKind::File, plain)), "file.temp");
        // Hidden wins over everything, directories included: that is the group order.
        assert_eq!(
            h.key(&entry(".git", EntryKind::Dir, EntryFlags::HIDDEN)),
            "file.hidden"
        );
        assert_eq!(
            h.key(&entry("run.sh", EntryKind::File, EntryFlags::HIDDEN)),
            "file.hidden"
        );
        // A broken link is ours rather than FAR's, and it is the loudest thing on the row.
        assert_eq!(
            h.key(&entry("gone", EntryKind::Symlink, EntryFlags::BROKEN_LINK)),
            "file.broken"
        );
    }

    #[test]
    fn the_executable_bit_counts_even_without_a_known_extension() {
        let h = Highlight::far_default();
        assert_eq!(
            h.key(&entry("scmd", EntryKind::File, EntryFlags::EXECUTABLE)),
            "file.exec"
        );
    }

    #[test]
    fn highlighting_can_be_turned_off_entirely() {
        let h = Highlight::none();
        let plain = EntryFlags::empty();
        assert_eq!(h.key(&entry("src.zip", EntryKind::File, plain)), "panel.fg");
        // The kinds that are not groups still show what they are.
        assert_eq!(h.key(&entry("src", EntryKind::Dir, plain)), "file.dir");
    }
}
