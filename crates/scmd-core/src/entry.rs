use std::borrow::Cow;
use std::ffi::OsString;
use std::time::SystemTime;

/// Stable identity of an entry, so a selection survives sorting, filtering and re-reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EntryId {
    /// Unix: (device, inode). Windows: (volume serial, file index).
    Fs(u64, u64),
    /// Fallback for providers without a stable id: hash of the name.
    Name(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EntryKind {
    /// The `..` pseudo entry; always first, never selectable.
    Parent,
    Dir,
    File,
    Symlink,
    Other,
}

impl EntryKind {
    pub fn is_dirlike(self) -> bool {
        matches!(self, EntryKind::Dir | EntryKind::Parent)
    }
}

/// Flags we can render without a second `stat`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct EntryFlags(u16);

impl EntryFlags {
    pub const HIDDEN: EntryFlags = EntryFlags(1 << 0);
    pub const READONLY: EntryFlags = EntryFlags(1 << 1);
    pub const EXECUTABLE: EntryFlags = EntryFlags(1 << 2);
    pub const SYSTEM: EntryFlags = EntryFlags(1 << 3);
    pub const BROKEN_LINK: EntryFlags = EntryFlags(1 << 4);

    pub const fn empty() -> Self {
        EntryFlags(0)
    }

    pub const fn bits(self) -> u16 {
        self.0
    }

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    pub fn remove(&mut self, other: Self) {
        self.0 &= !other.0;
    }

    pub fn set(&mut self, other: Self, on: bool) {
        if on {
            self.insert(other)
        } else {
            self.remove(other)
        }
    }
}

impl std::ops::BitOr for EntryFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        EntryFlags(self.0 | rhs.0)
    }
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub name: OsString,
    pub kind: EntryKind,
    pub size: u64,
    pub mtime: Option<SystemTime>,
    /// Creation time, and last access time. `None` means "this filesystem does not say", not
    /// "the epoch": `Metadata::created` is an error on ext3 and on most network mounts, and a
    /// VFS provider may carry neither field. A listing degrades to `None` rather than fail.
    pub btime: Option<SystemTime>,
    pub atime: Option<SystemTime>,
    pub flags: EntryFlags,
    pub id: EntryId,
    /// Resolved target of a symlink, when cheap to obtain.
    pub link_target: Option<OsString>,
}

impl Entry {
    pub fn parent_entry() -> Self {
        Entry {
            name: OsString::from(".."),
            kind: EntryKind::Parent,
            size: 0,
            mtime: None,
            btime: None,
            atime: None,
            flags: EntryFlags::empty(),
            id: EntryId::Name(0),
            link_target: None,
        }
    }

    pub fn name_str(&self) -> std::borrow::Cow<'_, str> {
        self.name.to_string_lossy()
    }

    /// Extension used for sorting and masks: empty for directories and for dotfiles.
    pub fn extension(&self) -> Cow<'_, str> {
        if self.kind.is_dirlike() {
            return Cow::Borrowed("");
        }
        match self.name.to_string_lossy() {
            Cow::Borrowed(s) => match s.rfind('.') {
                Some(0) | None => Cow::Borrowed(""),
                Some(i) => Cow::Borrowed(&s[i + 1..]),
            },
            Cow::Owned(s) => match s.rfind('.') {
                Some(0) | None => Cow::Owned(String::new()),
                Some(i) => Cow::Owned(s[i + 1..].to_owned()),
            },
        }
    }

    pub fn is_selectable(&self) -> bool {
        self.kind != EntryKind::Parent
    }
}
