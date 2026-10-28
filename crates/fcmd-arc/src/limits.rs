/// The guards every archive read goes through (SPEC 04 §4).
///
/// A zip bomb is a small file that claims to hold petabytes, or millions of entries, or the
/// same directory nested a thousand deep. None of that is rejected silently: the limits are
/// configuration, and the refusal says which one fired and what the archive asked for, so the
/// user can raise it for an archive they trust.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// How many entries one archive may list.
    pub entries: usize,
    /// The total uncompressed size the index may add up to.
    pub total_bytes: u64,
    /// Uncompressed over compressed. Text compresses 10:1 and a disk image far better, so the
    /// default is generous; a 1000:1 archive is a bomb or a curiosity.
    pub ratio: u64,
    /// How many containers deep a path may go.
    pub depth: usize,
    /// How large a container that is not a file on this disk may be before we refuse to copy
    /// it here (SPEC 04 §4). An archive inside an archive, or one on a server, has to be a
    /// real file before anything can read its index, and that copy is paid for in disk.
    pub spill_bytes: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            entries: 200_000,
            total_bytes: 64 << 30,
            ratio: 1_000,
            depth: 4,
            spill_bytes: 1 << 30,
        }
    }
}

impl Limits {
    /// Limits that refuse nothing, for a test or for a user who has decided.
    pub fn unlimited() -> Limits {
        Limits {
            entries: usize::MAX,
            total_bytes: u64::MAX,
            ratio: u64::MAX,
            depth: 64,
            spill_bytes: u64::MAX,
        }
    }
}
