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
    /// What it is a mount *of*: the device on Unix, the share behind a mapped letter on
    /// Windows. FAR calls it the associated path and shows it in a column of its own.
    pub device: Option<String>,
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
/// How long the dialog waits for one drive to describe itself before it lists it with what
/// is known and says which one kept it waiting.
pub const PATIENCE: std::time::Duration = std::time::Duration::from_millis(1500);

/// The drives, and the ones that did not answer within [`PATIENCE`]: a network letter whose
/// server has gone, a stale NFS mount, a card reader that spins up. Each drive is asked on a
/// thread of its own, so the wait is the slowest one's and never more than the patience —
/// not the sum of them all, which is what made the dialog take seconds to open.
pub fn list_timed() -> (Vec<Drive>, Vec<String>) {
    let (mut drives, late) = platform();
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        let home = home.to_string_lossy().into_owned();
        if !home.is_empty() && !drives.iter().any(|d| d.path == home) {
            let mut rows = vec![Drive {
                path: home,
                kind: Kind::Home,
                label: None,
                fs: None,
                device: None,
                total: None,
                free: None,
            }];
            fill_spaces(&mut rows);
            drives.extend(rows);
        }
    }
    (drives, late)
}

/// Ask `ask` about each item on a thread of its own, and keep what came back within the
/// patience; the items that did not answer are the second list.
fn in_parallel<T, R>(items: Vec<T>, ask: fn(&T) -> R) -> (Vec<(T, Option<R>)>, Vec<usize>)
where
    T: Send + Clone + 'static,
    R: Send + 'static,
{
    let (tx, rx) = std::sync::mpsc::channel();
    for (at, item) in items.iter().cloned().enumerate() {
        let tx = tx.clone();
        // Not scoped: a drive that never answers keeps its thread, and the dialog does not
        // wait for it.
        std::thread::spawn(move || {
            let _ = tx.send((at, ask(&item)));
        });
    }
    drop(tx);
    let deadline = std::time::Instant::now() + PATIENCE;
    let mut answers: Vec<Option<R>> = items.iter().map(|_| None).collect();
    let mut got = 0;
    while got < items.len() {
        let left = deadline.saturating_duration_since(std::time::Instant::now());
        match rx.recv_timeout(left) {
            Ok((at, r)) => {
                answers[at] = Some(r);
                got += 1;
            }
            Err(_) => break,
        }
    }
    let late = answers
        .iter()
        .enumerate()
        .filter(|(_, a)| a.is_none())
        .map(|(i, _)| i)
        .collect();
    (items.into_iter().zip(answers).collect(), late)
}

/// The sizes of every drive, each asked in parallel; one that does not answer in time keeps
/// its row, without sizes.
fn fill_spaces(drives: &mut [Drive]) -> Vec<String> {
    let paths: Vec<String> = drives.iter().map(|d| d.path.clone()).collect();
    let (answers, late) = in_parallel(paths, |p| fcmd_platform::disk(std::path::Path::new(p)));
    for (drive, (_, answer)) in drives.iter_mut().zip(answers) {
        if let Some(Some(d)) = answer {
            drive.total = Some(d.total);
            drive.free = Some(d.free);
        }
    }
    late.into_iter().map(|i| drives[i].path.clone()).collect()
}

#[cfg(target_os = "linux")]
fn platform() -> (Vec<Drive>, Vec<String>) {
    let text = std::fs::read_to_string("/proc/mounts").unwrap_or_default();
    let labels = linux_labels();
    let mut drives = parse_mounts(&text, &labels, &is_removable);
    // A mount whose server has gone answers `statvfs` after a timeout of its own, if at all.
    let late = fill_spaces(&mut drives);
    (drives, late)
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
fn platform() -> (Vec<Drive>, Vec<String>) {
    // `getmntinfo` is not in the crate we already depend on, and a dialog is not a reason to
    // add another: on macOS every mounted volume but the root one appears under `/Volumes`,
    // which is what the Finder shows too.
    let mut drives = vec![Drive {
        path: "/".to_string(),
        kind: Kind::Fixed,
        label: None,
        fs: None,
        device: None,
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
                device: None,
                total: None,
                free: None,
            });
        }
    }
    let late = fill_spaces(&mut drives);
    for drive in drives.iter_mut().filter(|d| !late.contains(&d.path)) {
        drive.fs = filesystem_name(&drive.path);
        if drive.fs.as_deref() == Some("msdos") || drive.fs.as_deref() == Some("exfat") {
            // Not proof of a stick, but on macOS it is what one almost always is.
            drive.kind = Kind::Removable;
        }
    }
    (drives, late)
}

#[cfg(windows)]
fn platform() -> (Vec<Drive>, Vec<String>) {
    // Every letter the system will describe, which is not the same as every letter that can
    // be opened: a CD drive with nothing in it answers `GetDriveTypeW` and refuses
    // everything else, and FAR lists it — that is what its "show CD-ROM" switch is about.
    //
    // The kind is asked first, of every letter: it answers at once. The rest — the label, the
    // file system, the share, the sizes — can wait for a server that is not there, so each
    // letter is asked on a thread of its own and one that is too slow is listed with what
    // its kind alone says.
    let kind_of = |k: fcmd_platform::VolumeKind| match k {
        fcmd_platform::VolumeKind::Removable => Some(Kind::Removable),
        fcmd_platform::VolumeKind::Fixed => Some(Kind::Fixed),
        fcmd_platform::VolumeKind::Network => Some(Kind::Network),
        fcmd_platform::VolumeKind::Optical => Some(Kind::Optical),
        fcmd_platform::VolumeKind::Memory => Some(Kind::Memory),
        // "Cannot say" and "there is no such root" are both "there is no drive here".
        fcmd_platform::VolumeKind::Unknown => None,
    };
    let letters: Vec<(String, Kind)> = (b'A'..=b'Z')
        .map(|letter| format!("{}:\\", letter as char))
        .filter_map(|path| {
            let kind = fcmd_platform::drive_kind(std::path::Path::new(&path)).and_then(kind_of)?;
            Some((path, kind))
        })
        .collect();
    let (answers, late) = in_parallel(letters, |(path, kind)| {
        let mut drive = Drive {
            path: path.clone(),
            kind: *kind,
            label: None,
            fs: None,
            device: None,
            total: None,
            free: None,
        };
        if let Some(volume) = fcmd_platform::volume(std::path::Path::new(path)) {
            drive.label = volume.label;
            drive.fs = volume.fs;
            // The share behind a mapped letter, which is the one thing a letter does not say
            // about itself.
            drive.device = volume.share;
            // Only a drive with something in it is asked how big it is. Asking an empty one
            // is what puts "There is no disk in the drive" on the screen, in a dialog box,
            // in front of whatever the user was doing.
            if volume.ready {
                fill_space(&mut drive);
            }
        }
        drive
    });
    let late_paths = late.iter().map(|&i| answers[i].0 .0.clone()).collect();
    let drives = answers
        .into_iter()
        .map(|((path, kind), answer)| {
            answer.unwrap_or(Drive {
                path,
                kind,
                label: None,
                fs: None,
                device: None,
                total: None,
                free: None,
            })
        })
        .collect();
    (drives, late_paths)
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn platform() -> (Vec<Drive>, Vec<String>) {
    let mut root = Drive {
        path: "/".to_string(),
        kind: Kind::Fixed,
        label: None,
        fs: None,
        device: None,
        total: None,
        free: None,
    };
    fill_space(&mut root);
    (vec![root], Vec::new())
}

/// The two sizes, through `fcmd-platform`, which is where the per-platform call lives and
/// the reason this one has no `#[cfg]` of its own any more. What is free is what an
/// unprivileged process may really use, as the panel frame already reports.
#[cfg_attr(any(target_os = "linux", target_os = "macos"), allow(dead_code))]
fn fill_space(drive: &mut Drive) {
    if let Some(d) = fcmd_platform::disk(std::path::Path::new(drive.path.as_str())) {
        drive.total = Some(d.total);
        drive.free = Some(d.free);
    }
}

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
            device: Some(device.clone()),
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

    /// One drive that does not answer keeps nobody waiting longer than the patience, and is
    /// named; the others are all there.
    #[test]
    fn a_drive_that_does_not_answer_is_named_and_not_waited_for() {
        let start = std::time::Instant::now();
        let (answers, late) = in_parallel(vec![1u64, 2, 30], |n| {
            std::thread::sleep(std::time::Duration::from_millis(*n * 100));
            *n * 10
        });
        assert!(start.elapsed() < PATIENCE + std::time::Duration::from_millis(500));
        assert_eq!(late, vec![2]);
        assert_eq!(answers[0].1, Some(10));
        assert_eq!(answers[1].1, Some(20));
        assert_eq!(answers[2].1, None);
    }

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
