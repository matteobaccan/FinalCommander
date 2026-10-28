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
    /// Windows' archive bit. It has no meaning on a Unix and is never set there; it is here
    /// because the attributes dialog offers it on Windows and has to show what the file
    /// actually carries rather than an empty box.
    pub const ARCHIVE: EntryFlags = EntryFlags(1 << 5);
    /// A link that points at a directory — a symbolic link on any platform, and a junction on
    /// Windows. It is *not* `EntryKind::Dir`, because the entry is the link and deleting it
    /// does not touch what it points at; but it opens as a directory and therefore sorts and
    /// reads as one. The listing already pays for the second `stat` to know whether a link is
    /// broken, so this costs nothing.
    pub const LINK_DIR: EntryFlags = EntryFlags(1 << 6);
    /// A directory somebody has measured, so `size` is what it holds rather than what the
    /// directory file itself costs. The flag and not a size of its own, because a directory
    /// *has* a size — 4 KiB of it on most Unix filesystems — and a panel that showed that
    /// figure would be answering a question nobody asked.
    pub const MEASURED: EntryFlags = EntryFlags(1 << 7);

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

    /// Does this entry *open* as a directory? A directory does, and so does a link that
    /// points at one — a symbolic link or a Windows junction. Everything that asks "where
    /// does the cursor go when this is opened", and therefore everything that groups the
    /// directories together, asks this rather than the kind: FAR sorts a junction among the
    /// folders because that is what it behaves like, and a user reading a `C:\` listing is
    /// looking for `Documents and Settings` up there and not down among the files.
    ///
    /// What it deliberately does not change is what the entry *is*: deleting a link deletes
    /// the link, and the name still carries its `@`.
    pub fn opens_as_dir(&self) -> bool {
        self.kind.is_dirlike() || self.flags.contains(EntryFlags::LINK_DIR)
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
