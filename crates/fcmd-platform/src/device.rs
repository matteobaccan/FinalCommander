//! What a raw device will say about itself (SPEC 05 §1.10).
//!
//! On Linux and macOS a disk is a file: `/dev/sdb` opens, seeks and reads like any other, and
//! `fcmd_core::device` measures it by seeking to the end. Windows is the exception this module
//! exists for, and it is two exceptions in one:
//!
//! - **The name.** There is no `/dev`. A disk is `\\.\PhysicalDrive0` and a volume is
//!   `\\.\C:` — paths the filesystem does not list, so nothing arrives at them from a panel;
//!   they are typed, which is also how `far:` and `hiew` get at them.
//! - **The length.** Seeking to the end of a device handle answers nothing useful, and its
//!   metadata says zero. The size has to be asked for: `IOCTL_DISK_GET_LENGTH_INFO`, which is
//!   what every disk tool on the platform uses.
//!
//! And one more thing that is not about measuring: such a handle refuses a read that does not
//! start on a sector boundary or is not a whole number of sectors long. So the sector size is
//! read here too, and `fcmd_core::device::read_at` does the arithmetic with it.
//!
//! Everywhere else this answers `None`, which is the honest answer: there is nothing here that
//! the ordinary file path does not already do better.

use std::path::Path;

/// What a device says about itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Geometry {
    /// How many bytes it holds, or `None` when the device will not say — a drive with no
    /// medium in it, a handle the call is refused on.
    pub bytes: Option<u64>,
    /// The block it insists on being read in. 512 or 4096 on anything current; never zero,
    /// because a caller dividing by it should not have to check.
    pub sector: u32,
}

/// Is this one of the names only Windows has for a device?
///
/// `\\.\PhysicalDrive0`, `\\.\C:`, `\\.\CdRom0`. Both slashes are accepted either way round,
/// because a path typed into a dialog on Windows arrives with whichever the typist used.
///
/// False on every other platform: there a device is a file and nothing about it is special.
pub fn is_device_path(path: &Path) -> bool {
    if !cfg!(windows) {
        return false;
    }
    let text = path.to_string_lossy().replace('/', "\\");
    // `\\.\x` is the shortest there is: the prefix and one character of name.
    text.len() > 4 && (text.starts_with("\\\\.\\") || text.starts_with("\\\\?\\"))
}

/// Ask the device how long it is and how it wants to be read.
///
/// The handle is the caller's, already open: opening a device twice is one refusal more than
/// necessary, and the caller needs the handle anyway. The path comes with it because only the
/// *name* says this is a device at all — a handle cannot be asked.
///
/// `None` when this is not a device name and on every platform but Windows. `Some` with a
/// `bytes` of `None` when the device would not say how long it is, which the caller shows as
/// a length it does not know rather than as zero.
pub fn geometry(path: &Path, file: &std::fs::File) -> Option<Geometry> {
    #[cfg(windows)]
    {
        windows_geometry(path, file)
    }
    #[cfg(not(windows))]
    {
        let _ = (path, file);
        None
    }
}

#[cfg(windows)]
fn windows_geometry(path: &Path, file: &std::fs::File) -> Option<Geometry> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::System::Ioctl::{
        DISK_GEOMETRY, GET_LENGTH_INFORMATION, IOCTL_DISK_GET_DRIVE_GEOMETRY,
        IOCTL_DISK_GET_LENGTH_INFO,
    };
    use windows_sys::Win32::System::IO::DeviceIoControl;

    if !is_device_path(path) {
        return None;
    }
    let handle = file.as_raw_handle() as HANDLE;

    // `DeviceIoControl` with a code that takes no input: a pointer to the answer, its size,
    // and somewhere to put how much of it was written.
    //
    // SAFETY: `handle` is open for the length of each call (the `File` outlives both), the
    // output buffer is a local of exactly the type the control code documents, and its size
    // is `size_of` that type.
    let mut bytes = None;
    let mut length = GET_LENGTH_INFORMATION::default();
    let mut written = 0u32;
    let ok = unsafe {
        DeviceIoControl(
            handle,
            IOCTL_DISK_GET_LENGTH_INFO,
            std::ptr::null(),
            0,
            (&raw mut length).cast(),
            std::mem::size_of::<GET_LENGTH_INFORMATION>() as u32,
            &raw mut written,
            std::ptr::null_mut(),
        )
    };
    if ok != 0 && length.Length > 0 {
        bytes = Some(length.Length as u64);
    }

    // The sector size. A volume answers this as readily as a disk does; a device that refuses
    // gets 512, which is what every such device has used for forty years and is the size the
    // call itself reports for anything that is not an advanced-format disk.
    let mut sector = 512u32;
    let mut disk = DISK_GEOMETRY::default();
    // SAFETY: as above.
    let ok = unsafe {
        DeviceIoControl(
            handle,
            IOCTL_DISK_GET_DRIVE_GEOMETRY,
            std::ptr::null(),
            0,
            (&raw mut disk).cast(),
            std::mem::size_of::<DISK_GEOMETRY>() as u32,
            &raw mut written,
            std::ptr::null_mut(),
        )
    };
    if ok != 0 && disk.BytesPerSector > 0 {
        sector = disk.BytesPerSector;
    }
    // A length the geometry knows and the length call did not: a floppy, a card reader with
    // nothing in it, a device where only one of the two is implemented.
    if bytes.is_none() && disk.BytesPerSector > 0 {
        let cylinders = disk.Cylinders.max(0) as u64;
        let per_cylinder = u64::from(disk.TracksPerCylinder) * u64::from(disk.SectorsPerTrack);
        let size = cylinders * per_cylinder * u64::from(disk.BytesPerSector);
        if size > 0 {
            bytes = Some(size);
        }
    }
    Some(Geometry { bytes, sector })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The name test is the one that runs everywhere, and it is the one that decides whether
    /// the rest of this is reached at all.
    #[test]
    fn only_windows_has_device_names() {
        let yes = [
            Path::new("\\\\.\\PhysicalDrive0"),
            Path::new("\\\\.\\C:"),
            Path::new("//./PhysicalDrive1"),
        ];
        let no = [
            Path::new("/dev/sda"),
            Path::new("C:\\Windows\\notepad.exe"),
            Path::new("\\\\server\\share\\file"),
            Path::new("\\\\.\\"),
            Path::new("plain.txt"),
        ];
        for path in yes {
            assert_eq!(is_device_path(path), cfg!(windows), "{}", path.display());
        }
        for path in no {
            assert!(!is_device_path(path), "{}", path.display());
        }
    }

    /// An ordinary file is never a device, and asking it for its geometry says so rather than
    /// inventing one. A real disk needs a machine and rights CI does not have; what can be
    /// pinned down here is that nothing else is mistaken for one.
    #[test]
    fn a_file_has_no_geometry() {
        let dir = std::env::temp_dir().join(format!("fcmd-dev-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a directory to work in");
        let path = dir.join("not-a-disk");
        std::fs::write(&path, b"0123456789").expect("written");
        let file = std::fs::File::open(&path).expect("opened");
        assert_eq!(geometry(&path, &file), None);
        drop(file);
        std::fs::remove_dir_all(&dir).ok();
    }
}
