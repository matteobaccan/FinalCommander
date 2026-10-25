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
