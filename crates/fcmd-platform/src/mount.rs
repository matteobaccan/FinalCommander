//! Which filesystem a directory is on, and how it was mounted (SPEC 02 §9).
//!
//! The information panel answers "where am I" with how much room is left, and that is half the
//! answer: the other half is *what* this is. A directory on `ext4` and one on a FUSE mount of
//! somebody's phone behave differently — one of them cannot set a time, the other cannot hold
//! a symbolic link — and when something is refused the reason is usually written in the mount
//! options. `ro` and `noexec` are the two that explain most refusals.
//!
//! Each platform keeps this somewhere else, and only one of the three keeps it in a file:
//!
//! - **Linux** writes the whole table in `/proc/self/mountinfo`, options and all, and that is
//!   read here with no call at all. The longest mount point that is a prefix of the path wins,
//!   which is what makes a bind mount inside another one answer for itself.
//! - **macOS** answers `statfs`, which gives the filesystem's name, what is mounted and a word
//!   of flags — fewer options than Linux's table, and the ones that matter are in it.
//! - **Windows** has no mount options. What it has is the filesystem's name, which
//!   [`super::volume`] already asks for, and that is what this reports there.
//!
//! Everything that fails answers `None`: a path that has gone away, a table that cannot be
//! read, a platform that does not answer. A panel with no mount line is a panel that does not
//! claim to know.

use std::path::{Path, PathBuf};

/// What a directory is sitting on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mount {
    /// What is mounted: a device, a share, or a pseudo-filesystem's own name.
    pub source: String,
    /// Where it is mounted. The deepest mount point that covers the path asked about.
    pub target: PathBuf,
    /// The filesystem's name, in the platform's own spelling: `ext4`, `apfs`, `NTFS`.
    pub fs: String,
    /// How it was mounted, in the platform's own words: `rw`, `noatime`, `nodev`. Empty where
    /// the platform does not keep any.
    pub options: Vec<String>,
}

impl Mount {
    /// Is this mounted read only? The one option worth a question of its own, because it is
    /// the answer to "why can I not write here".
    pub fn read_only(&self) -> bool {
        self.options.iter().any(|o| o == "ro")
    }

    /// The options as one line, which is how a panel shows them.
    pub fn options_line(&self) -> String {
        self.options.join(",")
    }
}

/// The mount a path is on, or `None` where this platform or this path will not say.
pub fn mount_of(path: &Path) -> Option<Mount> {
    let real = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    #[cfg(target_os = "linux")]
    {
        let table = std::fs::read_to_string("/proc/self/mountinfo")
            .ok()
            .and_then(|text| pick_mountinfo(&text, &real));
        if table.is_some() {
            return table;
        }
        // A container without `/proc/self` still usually has the older table, which has the
        // same answers in a different order and no bind-mount detail.
        let text = std::fs::read_to_string("/proc/mounts").ok()?;
        pick_mounts(&text, &real)
    }
    #[cfg(target_os = "macos")]
    {
        return statfs_mount(&real);
    }
    #[cfg(windows)]
    {
        // A drive letter's root is what Windows describes, so that is what is asked about.
        let root = real.components().next().map(|c| {
            let mut root = PathBuf::from(c.as_os_str());
            root.push("");
            root
        })?;
        let volume = super::volume(&root)?;
        return Some(Mount {
            source: volume.share.clone().unwrap_or_else(|| {
                volume
                    .label
                    .clone()
                    .unwrap_or_else(|| root.display().to_string())
            }),
            target: root,
            fs: volume.fs.clone()?,
            // Windows keeps no mount options. The volume's flags are a different question,
            // and the drives dialog is where they belong.
            options: Vec::new(),
        });
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    {
        let _ = real;
        None
    }
}

/// `/proc/self/mountinfo`, one line per mount:
///
/// ```text
/// 36 35 98:0 /mnt1 /mnt2 rw,noatime master:1 - ext3 /dev/root rw,errors=continue
/// ```
///
/// The fields up to the `-` are the mount's own, the ones after it the filesystem's. Between
/// them is a list of optional fields of unknown length, which is exactly why the separator is
/// there and why this splits on it rather than counting.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn pick_mountinfo(text: &str, path: &Path) -> Option<Mount> {
    let mut best: Option<Mount> = None;
    for line in text.lines() {
        let (left, right) = line.split_once(" - ")?;
        let mut fields = left.split(' ');
        let target = unescape(fields.nth(4)?);
        let mount_options = fields.next().unwrap_or("");
        let mut after = right.split(' ');
        let fs = after.next()?.to_string();
        let source = unescape(after.next().unwrap_or(""));
        let super_options = after.next().unwrap_or("");
        let target = PathBuf::from(target);
        if !covers(&target, path) {
            continue;
        }
        if best
            .as_ref()
            .is_some_and(|b| depth(&b.target) >= depth(&target))
        {
            continue;
        }
        best = Some(Mount {
            source,
            target,
            fs,
            options: merge_options(mount_options, super_options),
        });
    }
    best
}

/// `/proc/mounts`: `source target fstype options freq passno`, which is the same answer with
/// the bind-mount detail left out.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn pick_mounts(text: &str, path: &Path) -> Option<Mount> {
    let mut best: Option<Mount> = None;
    for line in text.lines() {
        let mut fields = line.split(' ');
        let source = unescape(fields.next()?);
        let target = PathBuf::from(unescape(fields.next()?));
        let fs = fields.next()?.to_string();
        let options = fields.next().unwrap_or("");
        if !covers(&target, path) {
            continue;
        }
        if best
            .as_ref()
            .is_some_and(|b| depth(&b.target) >= depth(&target))
        {
            continue;
        }
        best = Some(Mount {
            source,
            target,
            fs,
            options: merge_options(options, ""),
        });
    }
    best
}

/// The two option lists as one, in order, with what they repeat said once. `rw` appears in
/// both lists of nearly every mount, and twice is not more informative than once.
fn merge_options(first: &str, second: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for option in first
        .split(',')
        .chain(second.split(','))
        .map(str::trim)
        .filter(|o| !o.is_empty())
    {
        // `rw` and `ro` contradict rather than repeat: the super block's word is the one that
        // decides, and it comes second.
        if (option == "rw" && out.iter().any(|o| o == "ro"))
            || (option == "ro" && out.iter().any(|o| o == "rw"))
        {
            out.retain(|o| o != "rw" && o != "ro");
        }
        if !out.iter().any(|o| o == option) {
            out.push(option.to_string());
        }
    }
    out
}

/// Is `target` the mount point of `path`, or of a directory above it? Compared by component,
/// because `/media` is not the mount point of `/mediafiles`.
fn covers(target: &Path, path: &Path) -> bool {
    path.starts_with(target)
}

fn depth(path: &Path) -> usize {
    path.components().count()
}

/// `\040` and friends: the mount table escapes the characters that would otherwise end a
/// field, and a path with a space in it is not rare.
fn unescape(field: &str) -> String {
    if !field.contains('\\') {
        return field.to_string();
    }
    let bytes = field.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 3 < bytes.len() {
            let digits = &field[i + 1..i + 4];
            if let Ok(byte) = u8::from_str_radix(digits, 8) {
                out.push(byte);
                i += 4;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

#[cfg(target_os = "macos")]
fn statfs_mount(path: &Path) -> Option<Mount> {
    use std::ffi::{CStr, CString};
    use std::os::unix::ffi::OsStrExt as _;

    let c_path = CString::new(path.as_os_str().as_bytes()).ok()?;
    // SAFETY: a zeroed `statfs` is a valid one to write into, and the call only writes the
    // struct it is given and reads the NUL-terminated path.
    let mut stat: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statfs(c_path.as_ptr(), &mut stat) } != 0 {
        return None;
    }
    // SAFETY: both fields are NUL-terminated C strings written by the kernel.
    let fs = unsafe { CStr::from_ptr(stat.f_fstypename.as_ptr()) }
        .to_string_lossy()
        .to_string();
    let source = unsafe { CStr::from_ptr(stat.f_mntfromname.as_ptr()) }
        .to_string_lossy()
        .to_string();
    let target = unsafe { CStr::from_ptr(stat.f_mntonname.as_ptr()) }
        .to_string_lossy()
        .to_string();
    // The flags macOS has that are worth a word, in Linux's spelling where they mean the same
    // thing: somebody reading this panel on both machines should not have to learn two
    // vocabularies.
    let mut options = Vec::new();
    let flags = stat.f_flags as i32;
    options.push(
        if flags & libc::MNT_RDONLY != 0 {
            "ro"
        } else {
            "rw"
        }
        .to_string(),
    );
    for (flag, name) in [
        (libc::MNT_NOEXEC, "noexec"),
        (libc::MNT_NOSUID, "nosuid"),
        (libc::MNT_NODEV, "nodev"),
        (libc::MNT_SYNCHRONOUS, "sync"),
        (libc::MNT_NOATIME, "noatime"),
        (libc::MNT_UNION, "union"),
        (libc::MNT_EXPORTED, "exported"),
        (libc::MNT_QUARANTINE, "quarantine"),
        (libc::MNT_LOCAL, "local"),
        (libc::MNT_JOURNALED, "journaled"),
    ] {
        if flags & flag != 0 {
            options.push(name.to_string());
        }
    }
    Some(Mount {
        source,
        target: PathBuf::from(target),
        fs,
        options,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOUNTINFO: &str = "\
22 28 0:21 / /proc rw,nosuid,nodev,noexec,relatime shared:12 - proc proc rw
25 28 0:23 / /sys rw,nosuid,nodev,noexec,relatime shared:7 - sysfs sysfs rw
28 1 254:1 / / rw,relatime shared:1 - ext4 /dev/vda1 rw,errors=remount-ro
41 28 0:40 / /home/mb/mnt\\040point ro,relatime shared:21 - fuseblk /dev/sdb1 ro,user_id=0
44 28 254:1 /var/lib/docker /var/lib/docker rw,relatime shared:1 - ext4 /dev/vda1 rw
";

    #[test]
    fn the_deepest_mount_that_covers_the_path_is_the_answer() {
        // A file under `/` and nothing deeper: the root mount.
        let m = pick_mountinfo(MOUNTINFO, Path::new("/etc/hosts")).unwrap();
        assert_eq!(m.fs, "ext4");
        assert_eq!(m.source, "/dev/vda1");
        assert_eq!(m.target, Path::new("/"));

        // A bind mount of the same device, deeper: it answers for itself and not the root.
        let m = pick_mountinfo(MOUNTINFO, Path::new("/var/lib/docker/overlay2")).unwrap();
        assert_eq!(m.target, Path::new("/var/lib/docker"));

        // `/proc` is a mount of its own, and a pseudo-filesystem says what it is.
        let m = pick_mountinfo(MOUNTINFO, Path::new("/proc/1/status")).unwrap();
        assert_eq!(m.fs, "proc");
        assert!(m.options.iter().any(|o| o == "noexec"), "{:?}", m.options);
    }

    #[test]
    fn a_mount_point_with_a_space_in_it_is_read_as_one_path() {
        let m = pick_mountinfo(MOUNTINFO, Path::new("/home/mb/mnt point/photos")).unwrap();
        assert_eq!(m.target, Path::new("/home/mb/mnt point"));
        assert_eq!(m.fs, "fuseblk");
        assert!(m.read_only(), "{:?}", m.options);
    }

    #[test]
    fn the_two_option_lists_become_one_and_the_super_block_decides_read_only() {
        // `ro` on the mount and `rw` on the super block, which is a real combination: the
        // panel should say one of the two and not both.
        let text = "28 1 254:1 / / ro,relatime shared:1 - ext4 /dev/vda1 rw\n";
        let m = pick_mountinfo(text, Path::new("/etc")).unwrap();
        assert!(!m.read_only(), "the super block's word comes second");
        assert_eq!(m.options_line(), "relatime,rw");

        // And what the two lists repeat is said once.
        let text = "28 1 254:1 / / rw,noatime shared:1 - ext4 /dev/vda1 rw,noatime\n";
        let m = pick_mountinfo(text, Path::new("/")).unwrap();
        assert_eq!(m.options_line(), "rw,noatime");
    }

    #[test]
    fn the_older_table_answers_the_same_questions() {
        let text = "\
/dev/vda1 / ext4 rw,relatime 0 0
tmpfs /dev/shm tmpfs rw,nosuid,nodev 0 0
";
        let m = pick_mounts(text, Path::new("/dev/shm/thing")).unwrap();
        assert_eq!(m.fs, "tmpfs");
        assert_eq!(m.source, "tmpfs");
        assert!(m.options.iter().any(|o| o == "nosuid"));
    }

    #[test]
    fn a_table_that_is_not_one_answers_nothing_rather_than_guessing() {
        assert_eq!(pick_mountinfo("", Path::new("/")), None);
        assert_eq!(pick_mountinfo("rubbish\n", Path::new("/")), None);
        assert_eq!(pick_mounts("", Path::new("/")), None);
    }

    #[test]
    fn an_escape_is_the_byte_it_stands_for_and_a_backslash_is_itself() {
        assert_eq!(unescape("/mnt/a\\040b"), "/mnt/a b");
        assert_eq!(unescape("/mnt/plain"), "/mnt/plain");
        assert_eq!(unescape("/mnt/a\\134b"), "/mnt/a\\b");
        // A backslash that is not an escape is left where it is.
        assert_eq!(unescape("/mnt/a\\zz"), "/mnt/a\\zz");
    }

    /// The one test that asks the machine this is running on, which has a root filesystem
    /// whatever it is.
    #[test]
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn this_machine_answers_for_its_own_root() {
        let m = mount_of(Path::new("/")).expect("every unix knows what / is");
        assert!(!m.fs.is_empty(), "{m:?}");
        assert!(!m.options.is_empty(), "{m:?}");
    }
}
