//! The Unix half: `statvfs` for the device, and memory from wherever the system keeps it —
//! a file on Linux, a `sysctl` on macOS and the BSDs.

use std::path::Path;

use super::{Disk, Memory};

// `statvfs` does not agree with itself across the Unixes: the block counts are 64 bits on
// Linux and 32 on macOS, while the fragment size is 64 on both. The casts are what lets one
// expression serve either, and they are no-ops where the type is already wide enough — which
// is why clippy has to be told that here and nowhere else.
#[allow(clippy::unnecessary_cast)]
pub fn disk(path: &Path) -> Option<Disk> {
    let stat = nix::sys::statvfs::statvfs(path).ok()?;
    let unit = stat.fragment_size() as u64;
    Some(Disk {
        total: stat.blocks() as u64 * unit,
        // `blocks_available` is what an unprivileged process may actually use; the free count
        // includes the reserve only root can touch, which is not room to plan a copy against.
        free: stat.blocks_available() as u64 * unit,
        cluster: unit,
    })
}

#[cfg(target_os = "linux")]
pub fn memory() -> Option<Memory> {
    // `/proc/meminfo` in kibibytes, one `Name: value kB` per line. `MemAvailable` is the
    // kernel's own estimate of what a program could get without swapping, which is the
    // number worth showing — `MemFree` counts only what nothing is using at all, and on a
    // machine that has been up a while that is nearly zero however much is really free.
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    let field = |name: &str| -> Option<u64> {
        let line = text.lines().find(|l| l.starts_with(name))?;
        let value = line.split_whitespace().nth(1)?.parse::<u64>().ok()?;
        Some(value * 1024)
    };
    Some(Memory {
        total: field("MemTotal:")?,
        available: field("MemAvailable:"),
    })
}

#[cfg(target_os = "macos")]
pub fn memory() -> Option<Memory> {
    // `hw.memsize` is the one figure macOS gives without counting pages by hand: how much
    // the machine has. What is *available* needs `host_statistics64` and a reading of the
    // compressor, which is a lot of machinery for a line in a panel, so it says nothing.
    let mut bytes: u64 = 0;
    let mut len = std::mem::size_of::<u64>();
    let name = c"hw.memsize";
    // SAFETY: `sysctlbyname` writes at most `len` bytes into `bytes`, and `len` is the size
    // of what it points at. The name is a literal with its own NUL.
    let ok = unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            &mut bytes as *mut u64 as *mut libc::c_void,
            &mut len,
            std::ptr::null_mut(),
            0,
        )
    };
    (ok == 0 && bytes > 0).then_some(Memory {
        total: bytes,
        available: None,
    })
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn memory() -> Option<Memory> {
    None
}

/// The names of a file's owner and group.
///
/// The numbers come from the `lstat` the standard library already makes — the link's own
/// owner, not the owner of what it points at, because the row on the screen is the link.
/// Turning a number into a name goes through NSS, which may mean a file, a directory server
/// or nothing at all, so a number with no name behind it is shown as the number: an NFS mount
/// with no matching accounts is a normal thing to be looking at, and `4294967294` is a more
/// useful answer than a blank.
pub fn owner(path: &Path, cache: &mut super::OwnerCache) -> Option<super::Owner> {
    use std::os::unix::fs::MetadataExt;

    let meta = std::fs::symlink_metadata(path).ok()?;
    let (uid, gid) = (meta.uid(), meta.gid());
    let user = match cache.users.get(&uid) {
        Some(name) => name.clone(),
        None => {
            let name = nix::unistd::User::from_uid(nix::unistd::Uid::from_raw(uid))
                .ok()
                .flatten()
                .map(|u| u.name)
                .unwrap_or_else(|| uid.to_string());
            cache.users.insert(uid, name.clone());
            name
        }
    };
    let group = match cache.groups.get(&gid) {
        Some(name) => name.clone(),
        None => {
            let name = nix::unistd::Group::from_gid(nix::unistd::Gid::from_raw(gid))
                .ok()
                .flatten()
                .map(|g| g.name)
                .unwrap_or_else(|| gid.to_string());
            cache.groups.insert(gid, name.clone());
            name
        }
    };
    Some(super::Owner { user, group })
}

/// The number a user name stands for, and the same for a group.
///
/// A name goes through NSS, which is where `owner` read its names from, so what the dialog
/// shows is what these take back. A string of digits is taken as the number itself and not
/// looked up at all: a filesystem with no matching accounts shows `4294967294`, and typing
/// that back has to mean that id rather than nothing. `None` for a name nothing knows, which
/// the caller turns into a message rather than into a `chown` to id zero.
pub fn user_id(name: &str) -> Option<u32> {
    if let Ok(id) = name.parse::<u32>() {
        return Some(id);
    }
    nix::unistd::User::from_name(name)
        .ok()
        .flatten()
        .map(|u| u.uid.as_raw())
}

pub fn group_id(name: &str) -> Option<u32> {
    if let Ok(id) = name.parse::<u32>() {
        return Some(id);
    }
    nix::unistd::Group::from_name(name)
        .ok()
        .flatten()
        .map(|g| g.gid.as_raw())
}

/// Hand a file to somebody else: `chown`, with whichever of the two was asked for.
///
/// The link itself, not what it points at — `chown -h`. The row on the screen was read with
/// an `lstat` and shows the link's own owner, so changing the target's would be answering a
/// question nobody asked and leaving the answer off the screen.
///
/// What the kernel refuses comes back as it refused it. Giving a file away needs a privilege
/// an ordinary user does not have, and a group can only be set to one the user is in; neither
/// is worth pretending about, and neither is worth asking for a password over — `sudo` is one
/// line away in the command line below the panels, where the user can see what it is for.
pub fn set_owner(path: &Path, uid: Option<u32>, gid: Option<u32>) -> std::io::Result<()> {
    use nix::unistd::{Gid, Uid};
    nix::unistd::fchownat(
        nix::fcntl::AT_FDCWD,
        path,
        uid.map(Uid::from_raw),
        gid.map(Gid::from_raw),
        nix::fcntl::AtFlags::AT_SYMLINK_NOFOLLOW,
    )
    .map_err(std::io::Error::from)
}
