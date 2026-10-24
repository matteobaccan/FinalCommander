//! The drives a panel can be sent to: what is mounted, what kind of thing it is, and how
//! much room is left on it.
//!
//! FAR's change-drive dialog (`Alt+F1`, `Alt+F2`) is a table rather than a list of paths,
//! and the columns are the reason it is useful: the type says whether a thing is a disk, a
//! stick, a share or a filesystem that lives in memory, and the two sizes say at a glance
//! where a copy will fit. This module answers that question per platform and says nothing it
//! has not actually read — a column it cannot fill stays empty rather than guessing.

// Only the mount table and its labels use a map, and only two platforms have one.
#[cfg(any(target_os = "linux", test))]
use std::collections::HashMap;

/// One row of the change-drive dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drive {
    /// Where the panel goes.
    pub path: String,
    pub kind: Kind,
    /// The volume's name, when the system has one for it.
    pub label: Option<String>,
    /// `ext4`, `apfs`, `ntfs`, `nfs`…
    pub fs: Option<String>,
    pub total: Option<u64>,
    pub free: Option<u64>,
}

/// What sort of thing a drive is. The word in the first column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Fixed,
    Removable,
    Optical,
    Network,
    /// A filesystem that lives in memory: `tmpfs`, `ramfs`. Writing there is not storage.
    Memory,
    /// Where the user's own files are. Not a device, but it is the row most often wanted.
    Home,
    /// An open connection, which is a drive as far as a panel is concerned.
    Connection,
}

impl Kind {
    /// The message id for the word shown in the type column.
    pub fn label_key(self) -> &'static str {
        match self {
            Kind::Fixed => "drive-fixed",
            Kind::Removable => "drive-removable",
            Kind::Optical => "drive-optical",
            Kind::Network => "drive-network",
            Kind::Memory => "drive-memory",
            Kind::Home => "drive-home",
            Kind::Connection => "drive-connection",
        }
    }
}

/// Everything mounted, as the dialog shows it.
pub fn list() -> Vec<Drive> {
    let mut drives = platform();
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        let home = home.to_string_lossy().into_owned();
        if !home.is_empty() && !drives.iter().any(|d| d.path == home) {
            let mut row = Drive {
                path: home,
                kind: Kind::Home,
                label: None,
                fs: None,
                total: None,
                free: None,
            };
            fill_space(&mut row);
            drives.push(row);
        }
    }
    drives
}

#[cfg(target_os = "linux")]
fn platform() -> Vec<Drive> {
    let text = std::fs::read_to_string("/proc/mounts").unwrap_or_default();
    let labels = linux_labels();
    let mut drives = parse_mounts(&text, &labels, &is_removable);
    for drive in &mut drives {
        fill_space(drive);
    }
    drives
}

/// `/dev/disk/by-label/Backup -> ../../sdb1`, read back as `/dev/sdb1 -> Backup`.
#[cfg(target_os = "linux")]
fn linux_labels() -> HashMap<String, String> {
    let mut out = HashMap::new();
    let Ok(entries) = std::fs::read_dir("/dev/disk/by-label") else {
        return out;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        // The names are escaped the same way the mount table escapes them.
        let name = unescape(&name);
        if let Ok(target) = std::fs::canonicalize(entry.path()) {
            out.insert(target.to_string_lossy().into_owned(), name);
        }
    }
    out
}

#[cfg(target_os = "macos")]
fn platform() -> Vec<Drive> {
    // `getmntinfo` is not in the crate we already depend on, and a dialog is not a reason to
    // add another: on macOS every mounted volume but the root one appears under `/Volumes`,
    // which is what the Finder shows too.
    let mut drives = vec![Drive {
        path: "/".to_string(),
        kind: Kind::Fixed,
        label: None,
        fs: None,
        total: None,
        free: None,
    }];
    if let Ok(entries) = std::fs::read_dir("/Volumes") {
        let mut volumes: Vec<_> = entries
            .flatten()
            .filter(|e| e.path().is_dir())
            .map(|e| {
                (
                    e.path().to_string_lossy().into_owned(),
                    e.file_name().to_string_lossy().into_owned(),
                )
            })
            .collect();
        volumes.sort();
        for (path, label) in volumes {
            drives.push(Drive {
                path,
                kind: Kind::Fixed,
                label: Some(label),
                fs: None,
                total: None,
                free: None,
            });
        }
    }
    for drive in &mut drives {
        fill_space(drive);
        drive.fs = filesystem_name(&drive.path);
        if drive.fs.as_deref() == Some("msdos") || drive.fs.as_deref() == Some("exfat") {
            // Not proof of a stick, but on macOS it is what one almost always is.
            drive.kind = Kind::Removable;
        }
    }
    drives
}

#[cfg(windows)]
fn platform() -> Vec<Drive> {
    // The letters that exist. Saying more — the kind, the label, the two sizes — means
    // `GetDriveTypeW` and friends, and a Windows API crate we do not depend on yet; the
    // dialog shows what it knows and leaves the rest of the row empty.
    (b'A'..=b'Z')
        .map(|letter| format!("{}:\\", letter as char))
        .filter(|path| std::path::Path::new(path).exists())
        .map(|path| Drive {
            path,
            kind: Kind::Fixed,
            label: None,
            fs: None,
            total: None,
            free: None,
        })
        .collect()
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn platform() -> Vec<Drive> {
    let mut root = Drive {
        path: "/".to_string(),
        kind: Kind::Fixed,
        label: None,
        fs: None,
        total: None,
        free: None,
    };
    fill_space(&mut root);
    vec![root]
}

/// The two sizes, from `statvfs`. Unavailable on Windows, where both columns stay empty.
///
/// `statvfs` is an old interface and every system chose its own widths for it: the counts are
/// 64-bit on Linux and 32-bit on macOS, so the conversion that looks useless on one is what
/// makes it compile on the other.
#[cfg(unix)]
#[allow(clippy::useless_conversion)]
fn fill_space(drive: &mut Drive) {
    let Ok(stat) = nix::sys::statvfs::statvfs(drive.path.as_str()) else {
        return;
    };
    let block = u64::from(stat.fragment_size());
    drive.total = Some(u64::from(stat.blocks()) * block);
    // What an unprivileged process may really use, as the panel frame already reports.
    drive.free = Some(u64::from(stat.blocks_available()) * block);
}

#[cfg(not(unix))]
fn fill_space(_drive: &mut Drive) {}

#[cfg(target_os = "macos")]
fn filesystem_name(path: &str) -> Option<String> {
    let stat = nix::sys::statfs::statfs(path).ok()?;
    Some(stat.filesystem_type_name().to_string())
}

/// The mount table as the dialog's rows: one line is
/// `device mountpoint fstype options dump pass`, with spaces and tabs written as `\040`
/// and `\011`.
///
/// Pseudo filesystems are left out. They are mounted, but none of them is a place to put a
/// file, and a dialog with forty rows of `cgroup` in it is a dialog nobody reads.
#[cfg(any(target_os = "linux", test))]
fn parse_mounts(
    text: &str,
    labels: &HashMap<String, String>,
    removable: &dyn Fn(&str) -> bool,
) -> Vec<Drive> {
    let mut out: Vec<Drive> = Vec::new();
    for line in text.lines() {
        let mut fields = line.split_whitespace();
        let (Some(device), Some(point), Some(fs)) = (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        if is_pseudo(fs) || is_plumbing(point) {
            continue;
        }
        let device = unescape(device);
        let point = unescape(point);
        // The same device mounted twice is one drive: the first mount point is the one the
        // system itself uses.
        if out.iter().any(|d| d.path == point) {
            continue;
        }
        out.push(Drive {
            kind: kind_of(fs, &device, removable),
            label: labels.get(&device).cloned(),
            fs: Some(fs.to_string()),
            path: point,
            total: None,
            free: None,
        });
    }
    // In a settled order rather than the order the kernel happens to list them in, with the
    // root first because it is the one that is always there.
    out.sort_by(|a, b| match (a.path.as_str(), b.path.as_str()) {
        ("/", "/") => std::cmp::Ordering::Equal,
        ("/", _) => std::cmp::Ordering::Less,
        (_, "/") => std::cmp::Ordering::Greater,
        (x, y) => x.cmp(y),
    });
    out
}

/// A mount point that is part of how the system runs rather than somewhere to put a file.
///
/// The filesystem type is not enough on its own: a `tmpfs` under `/run` or `/sys/fs/cgroup`
/// is plumbing, while a `tmpfs` someone mounted on `/mnt/fast` is a place they chose. The
/// rule is where it is mounted, not what it is.
#[cfg(any(target_os = "linux", test))]
fn is_plumbing(point: &str) -> bool {
    const UNDER: [&str; 5] = ["/proc", "/sys", "/dev", "/run", "/var/lib/docker"];
    UNDER
        .iter()
        .any(|prefix| point == *prefix || point.starts_with(&format!("{prefix}/")))
}

/// A filesystem the kernel makes up rather than one holding files of ours.
#[cfg(any(target_os = "linux", test))]
fn is_pseudo(fs: &str) -> bool {
    const PSEUDO: [&str; 22] = [
        "autofs",
        "bpf",
        "binfmt_misc",
        "cgroup",
        "cgroup2",
        "configfs",
        "debugfs",
        "devpts",
        "devtmpfs",
        "efivarfs",
        "fuse.gvfsd-fuse",
        "fusectl",
        "hugetlbfs",
        "mqueue",
        "proc",
        "pstore",
        "securityfs",
        "selinuxfs",
        "squashfs",
        "sysfs",
        "tracefs",
        "rpc_pipefs",
    ];
    PSEUDO.contains(&fs)
}

#[cfg(any(target_os = "linux", test))]
fn kind_of(fs: &str, device: &str, removable: &dyn Fn(&str) -> bool) -> Kind {
    const NETWORK: [&str; 10] = [
        "nfs",
        "nfs4",
        "cifs",
        "smbfs",
        "smb3",
        "afs",
        "ncpfs",
        "9p",
        "fuse.sshfs",
        "ceph",
    ];
    const OPTICAL: [&str; 2] = ["iso9660", "udf"];
    if NETWORK.contains(&fs) {
        return Kind::Network;
    }
    if OPTICAL.contains(&fs) {
        return Kind::Optical;
    }
    if fs == "tmpfs" || fs == "ramfs" {
        return Kind::Memory;
    }
    if removable(device) {
        return Kind::Removable;
    }
    Kind::Fixed
}

/// `/sys/block/sdb/removable` is `1` for a stick and `0` for a disk. A partition's own
/// directory has no such file, so the digits come off the name first.
///
/// It is passed in rather than called directly so that the parsing can be tested without a
/// `/sys` to read: on a machine with no such device the honest answer is "not removable",
/// and a test that depended on this machine's disks would say nothing about the parser.
#[cfg(any(target_os = "linux", test))]
fn is_removable(device: &str) -> bool {
    let Some(name) = device.strip_prefix("/dev/") else {
        return false;
    };
    let whole = name.trim_end_matches(|c: char| c.is_ascii_digit());
    // An NVMe partition is `nvme0n1p2`: the partition suffix is `p` plus digits.
    let whole = whole.strip_suffix('p').unwrap_or(whole);
    std::fs::read_to_string(format!("/sys/block/{whole}/removable"))
        .map(|text| text.trim() == "1")
        .unwrap_or(false)
}

/// `\040` and friends, as the mount table writes a space.
#[cfg(any(target_os = "linux", test))]
fn unescape(raw: &str) -> String {
    if !raw.contains('\\') {
        return raw.to_string();
    }
    let mut out = String::with_capacity(raw.len());
    let bytes = raw.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 3 < bytes.len() {
            let digits = &raw[i + 1..i + 4];
            if let Ok(code) = u8::from_str_radix(digits, 8) {
                out.push(code as char);
                i += 4;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mount_table_becomes_the_rows_of_the_dialog() {
        let text = "\
sysfs /sys sysfs rw,nosuid 0 0
proc /proc proc rw,nosuid 0 0
/dev/nvme0n1p2 / ext4 rw,relatime 0 0
/dev/nvme0n1p1 /boot/efi vfat rw,relatime 0 0
tmpfs /dev/shm tmpfs rw,nosuid 0 0
tmpfs /run/user/1000 tmpfs rw,nosuid 0 0
tmpfs /mnt/fast tmpfs rw,nosuid 0 0
/dev/sdb1 /media/My\\040Stick exfat rw,nosuid 0 0
fileserver:/export /mnt/share nfs4 rw,relatime 0 0
/dev/sr0 /media/cdrom iso9660 ro,nosuid 0 0
";
        let mut labels = HashMap::new();
        labels.insert("/dev/nvme0n1p2".to_string(), "system".to_string());
        // The one device this machine would have to be asked about, answered here.
        let removable = |device: &str| device == "/dev/sdb1";
        let drives = parse_mounts(text, &labels, &removable);
        let paths: Vec<&str> = drives.iter().map(|d| d.path.as_str()).collect();
        assert_eq!(
            paths,
            [
                "/",
                "/boot/efi",
                "/media/My Stick",
                "/media/cdrom",
                "/mnt/fast",
                "/mnt/share",
            ],
            "the pseudo filesystems and the plumbing should not be here"
        );
        let by_path = |want: &str| drives.iter().find(|d| d.path == want).expect(want);
        assert_eq!(drives[0].fs.as_deref(), Some("ext4"));
        assert_eq!(drives[0].label.as_deref(), Some("system"));
        assert_eq!(drives[0].kind, Kind::Fixed);
        assert_eq!(by_path("/media/My Stick").kind, Kind::Removable);
        assert_eq!(by_path("/mnt/share").kind, Kind::Network);
        assert_eq!(by_path("/media/cdrom").kind, Kind::Optical);
        // A tmpfs is plumbing under `/run` and a choice under `/mnt`: where it is mounted
        // decides, not what it is.
        assert_eq!(by_path("/mnt/fast").kind, Kind::Memory);
    }

    #[test]
    fn the_same_place_is_listed_once() {
        let text = "\
/dev/sda1 /data ext4 rw 0 0
/dev/sda1 /data ext4 ro,remount 0 0
";
        assert_eq!(parse_mounts(text, &HashMap::new(), &|_| false).len(), 1);
    }

    #[test]
    fn an_octal_escape_that_is_not_one_is_left_alone() {
        assert_eq!(unescape("/mnt/plain"), "/mnt/plain");
        assert_eq!(unescape("/mnt/a\\040b"), "/mnt/a b");
        assert_eq!(unescape("/mnt/back\\slash"), "/mnt/back\\slash");
    }
}
