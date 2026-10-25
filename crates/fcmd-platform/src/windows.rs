//! The Windows half: three calls, each one the only way to get what it answers.

use std::os::windows::ffi::OsStrExt;
use std::path::{Component, Path, PathBuf};

use windows_sys::Win32::Storage::FileSystem::{GetDiskFreeSpaceExW, GetDiskFreeSpaceW};
use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

use super::{Disk, Memory};

/// A path as Windows wants it: UTF-16 with a NUL after it.
fn wide(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}

/// The root of the volume a path is on — `C:\`, or `\\server\share\` — which is what
/// `GetDiskFreeSpaceW` insists on being given. `None` for a relative path, which has no
/// volume of its own to name.
fn volume_root(path: &Path) -> Option<PathBuf> {
    let mut parts = path.components();
    let prefix = match parts.next()? {
        Component::Prefix(p) => p,
        _ => return None,
    };
    let mut root = PathBuf::from(prefix.as_os_str());
    root.push(std::path::MAIN_SEPARATOR_STR);
    Some(root)
}

pub fn disk(path: &Path) -> Option<Disk> {
    let name = wide(path);
    let mut free_to_caller: u64 = 0;
    let mut total: u64 = 0;
    let mut free: u64 = 0;
    // SAFETY: `name` is NUL-terminated and lives across the call; the three pointers are to
    // locals of the width the call writes.
    let ok =
        unsafe { GetDiskFreeSpaceExW(name.as_ptr(), &mut free_to_caller, &mut total, &mut free) };
    if ok == 0 || total == 0 {
        return None;
    }
    // The allocation unit, from the volume's root. A quota can make the first call answer
    // about the user rather than the volume; this one is about the volume either way, and a
    // failure here costs the cluster size and nothing else.
    let mut cluster = 0u64;
    if let Some(root) = volume_root(path) {
        let root = wide(&root);
        let (mut sectors, mut bytes, mut free_clusters, mut clusters) = (0u32, 0u32, 0u32, 0u32);
        // SAFETY: as above — a NUL-terminated name and four `u32` locals.
        let ok = unsafe {
            GetDiskFreeSpaceW(
                root.as_ptr(),
                &mut sectors,
                &mut bytes,
                &mut free_clusters,
                &mut clusters,
            )
        };
        if ok != 0 {
            cluster = u64::from(sectors) * u64::from(bytes);
        }
    }
    Some(Disk {
        total,
        // What *this* user may write, which is what the panel frame is about: a quota makes
        // it smaller than the volume's own free space, and the smaller number is the true one.
        free: free_to_caller.min(free),
        cluster,
    })
}

pub fn memory() -> Option<Memory> {
    let mut status: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
    status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
    // SAFETY: the struct is zeroed and its `dwLength` says how big it is, which is the whole
    // contract of the call.
    let ok = unsafe { GlobalMemoryStatusEx(&mut status) };
    (ok != 0 && status.ullTotalPhys > 0).then_some(Memory {
        total: status.ullTotalPhys,
        available: Some(status.ullAvailPhys),
    })
}
