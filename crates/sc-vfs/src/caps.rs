/// What a provider can actually do. The UI asks before it offers: no "set permissions" entry
/// on a path whose provider lacks `META_MODE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Caps(u32);

impl Caps {
    pub const READ: Caps = Caps(1 << 0);
    pub const WRITE: Caps = Caps(1 << 1);
    pub const CREATE: Caps = Caps(1 << 2);
    pub const DELETE: Caps = Caps(1 << 3);
    pub const RENAME: Caps = Caps(1 << 4);
    pub const SEEK: Caps = Caps(1 << 5);
    pub const TRUNCATE: Caps = Caps(1 << 6);
    pub const META_TIMES: Caps = Caps(1 << 7);
    pub const META_MODE: Caps = Caps(1 << 8);
    pub const META_OWNER: Caps = Caps(1 << 9);
    pub const XATTR: Caps = Caps(1 << 10);
    pub const STREAMS: Caps = Caps(1 << 11);
    pub const LINKS: Caps = Caps(1 << 12);
    pub const HARDLINKS: Caps = Caps(1 << 13);
    pub const WATCH: Caps = Caps(1 << 14);
    pub const SERVER_SIDE_COPY: Caps = Caps(1 << 15);
    pub const RESUME: Caps = Caps(1 << 16);
    pub const RANDOM_WRITE: Caps = Caps(1 << 17);
    pub const CASE_SENSITIVE: Caps = Caps(1 << 18);
    pub const ATOMIC_RENAME: Caps = Caps(1 << 19);

    pub const fn empty() -> Self {
        Caps(0)
    }

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub const fn union(self, other: Self) -> Self {
        Caps(self.0 | other.0)
    }

    pub const fn bits(self) -> u32 {
        self.0
    }
}

impl std::ops::BitOr for Caps {
    type Output = Caps;

    fn bitor(self, rhs: Caps) -> Caps {
        self.union(rhs)
    }
}
