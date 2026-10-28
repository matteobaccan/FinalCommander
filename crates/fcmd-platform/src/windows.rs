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

/// The names of a file's owner and its group.
///
/// Windows keeps neither as a number on the file: the security descriptor holds a SID per
/// side, and turning a SID into a name is a lookup against the machine or the domain. So this
/// is two calls, both of which can only be made one file at a time — which is why the owners
/// view asks for them in a worker and not while drawing.
///
/// Everything that fails answers `None` rather than a guess: a file on a filesystem with no
/// security at all (a FAT stick) has no owner to report, and a SID the domain will not resolve
/// is left as the SID's own string so that the row says *something* true.
/// How many names a file has, which on Windows means opening it and asking.
///
/// NTFS keeps the count in the file record and `GetFileInformationByHandle` is the way to it;
/// there is no cheaper call, which is why this is a lookup per row and happens in a worker
/// while the links view is the one on screen. A file nobody may open — in use, or not ours to
/// read — says nothing rather than guessing at one.
pub fn links(path: &Path) -> Option<u64> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_DELETE,
        FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };

    let name = wide(path);
    // SAFETY: `name` is NUL-terminated and lives across the call. No access is asked for —
    // zero is enough to read metadata — so this opens a file that is already open for
    // writing elsewhere. `BACKUP_SEMANTICS` lets a directory be opened at all and
    // `OPEN_REPARSE_POINT` makes the link itself the thing opened, not its target.
    let handle = unsafe {
        CreateFileW(
            name.as_ptr(),
            0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            std::ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return None;
    }
    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    // SAFETY: the handle is open and `info` is a whole, zeroed structure of the type the
    // call fills in.
    let ok = unsafe { GetFileInformationByHandle(handle, &mut info) };
    // SAFETY: the handle came from `CreateFileW` above and is closed exactly once.
    unsafe { CloseHandle(handle) };
    (ok != 0).then_some(info.nNumberOfLinks as u64)
}

/// Everything running, from a snapshot of the process table.
///
/// `CreateToolhelp32Snapshot` is the call that needs no privilege: it gives the name, the pid
/// and the thread count of every process the user may see. The memory needs a second call per
/// process and a handle we may not get — a system process refuses — so a process that will
/// not answer keeps its row with an empty memory column rather than being left out.
pub fn processes() -> Vec<super::Process> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };

    // SAFETY: the call takes no pointer of ours and returns a handle or the invalid value.
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut entry: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
    entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
    // SAFETY: `entry` is a whole, zeroed structure whose `dwSize` says how big it is, which
    // is what the call reads before writing into it.
    let mut ok = unsafe { Process32FirstW(snapshot, &mut entry) };
    while ok != 0 {
        let name = {
            let raw = &entry.szExeFile;
            let end = raw.iter().position(|c| *c == 0).unwrap_or(raw.len());
            String::from_utf16_lossy(&raw[..end])
        };
        out.push(super::Process {
            pid: entry.th32ProcessID,
            name,
            memory: memory_of(entry.th32ProcessID),
            threads: entry.cntThreads,
            bits: 0,
        });
        // SAFETY: as above; the snapshot is still open.
        ok = unsafe { Process32NextW(snapshot, &mut entry) };
    }
    // SAFETY: the handle came from `CreateToolhelp32Snapshot` and is closed exactly once.
    unsafe { CloseHandle(snapshot) };
    out
}

/// What one process has resident, or zero when it will not say.
#[cfg(windows)]
fn memory_of(pid: u32) -> u64 {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

    // SAFETY: no pointer of ours is passed; the result is a handle or null.
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        return 0;
    }
    let mut counters: PROCESS_MEMORY_COUNTERS = unsafe { std::mem::zeroed() };
    counters.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
    // SAFETY: the handle is open and `counters` is a whole structure of the size its own
    // `cb` field declares, which is what the call fills in.
    let ok = unsafe {
        GetProcessMemoryInfo(
            handle,
            &mut counters,
            std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        )
    };
    // SAFETY: the handle came from `OpenProcess` and is closed exactly once.
    unsafe { CloseHandle(handle) };
    if ok == 0 {
        0
    } else {
        counters.WorkingSetSize as u64
    }
}

pub fn owner(path: &Path, cache: &mut super::OwnerCache) -> Option<super::Owner> {
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Authorization::{GetNamedSecurityInfoW, SE_FILE_OBJECT};
    use windows_sys::Win32::Security::{
        GROUP_SECURITY_INFORMATION, OWNER_SECURITY_INFORMATION, PSID,
    };

    let name = wide(path);
    let mut sid_owner: PSID = std::ptr::null_mut();
    let mut sid_group: PSID = std::ptr::null_mut();
    let mut descriptor = std::ptr::null_mut();
    // SAFETY: `name` is NUL-terminated and lives across the call. The two SID pointers and
    // the descriptor are out parameters; the three the call does not need are null, which is
    // what it documents as "do not return this". On success the descriptor is ours to free
    // with `LocalFree`, and the SIDs point into it — so they are read before it is freed.
    let status = unsafe {
        GetNamedSecurityInfoW(
            name.as_ptr(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION,
            &mut sid_owner,
            &mut sid_group,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut descriptor,
        )
    };
    if status != 0 {
        return None;
    }
    let user = account_name(sid_owner, cache);
    let group = account_name(sid_group, cache);
    if !descriptor.is_null() {
        // SAFETY: the descriptor was allocated by the call above and has not been freed.
        unsafe { LocalFree(descriptor) };
    }
    Some(super::Owner {
        user: user.unwrap_or_default(),
        group: group.unwrap_or_default(),
    })
}

/// The account a SID names, as `DOMAIN\name` where there is a domain. Cached by the SID's own
/// string form, because a directory listing is mostly one or two accounts repeated and the
/// lookup can cross a network.
fn account_name(
    sid: windows_sys::Win32::Security::PSID,
    cache: &mut super::OwnerCache,
) -> Option<String> {
    use windows_sys::Win32::Security::Authorization::ConvertSidToStringSidW;
    use windows_sys::Win32::Security::LookupAccountSidW;

    if sid.is_null() {
        return None;
    }
    // The SID as text is both the cache key and the answer of last resort.
    let mut text_ptr: *mut u16 = std::ptr::null_mut();
    // SAFETY: `sid` is non-null and points into a descriptor that is still alive; the out
    // parameter is a local pointer the call allocates for us to free.
    let ok = unsafe { ConvertSidToStringSidW(sid, &mut text_ptr) };
    let key = if ok != 0 && !text_ptr.is_null() {
        // SAFETY: the call wrote a NUL-terminated UTF-16 string at `text_ptr`.
        let text = unsafe { from_wide(text_ptr) };
        // SAFETY: `text_ptr` came from the call above and has not been freed.
        unsafe { windows_sys::Win32::Foundation::LocalFree(text_ptr.cast()) };
        text
    } else {
        String::new()
    };
    if let Some(found) = cache.sids.get(&key) {
        return Some(found.clone());
    }

    let mut name = [0u16; 256];
    let mut name_len = name.len() as u32;
    let mut domain = [0u16; 256];
    let mut domain_len = domain.len() as u32;
    let mut kind = 0i32;
    // SAFETY: both buffers are 256 wide characters long and both lengths say so; the call
    // writes at most that many and reports what it needed if it wanted more, in which case
    // it fails and the SID's own text is used instead.
    let ok = unsafe {
        LookupAccountSidW(
            std::ptr::null(),
            sid,
            name.as_mut_ptr(),
            &mut name_len,
            domain.as_mut_ptr(),
            &mut domain_len,
            &mut kind,
        )
    };
    let resolved = if ok != 0 {
        let name = String::from_utf16_lossy(&name[..name_len as usize]);
        let domain = String::from_utf16_lossy(&domain[..domain_len as usize]);
        if domain.is_empty() {
            name
        } else {
            format!("{domain}\\{name}")
        }
    } else if key.is_empty() {
        return None;
    } else {
        key.clone()
    };
    if !key.is_empty() {
        cache.sids.insert(key, resolved.clone());
    }
    Some(resolved)
}

/// A NUL-terminated UTF-16 string Windows allocated, as a `String`.
///
/// # Safety
///
/// `ptr` must point at a NUL-terminated sequence of `u16` that stays valid for the call.
unsafe fn from_wide(ptr: *const u16) -> String {
    let mut len = 0;
    // SAFETY: the caller promises a NUL-terminated string, so the walk stops inside it.
    while unsafe { *ptr.add(len) } != 0 {
        len += 1;
    }
    // SAFETY: `len` characters were just walked, so the slice is inside the string.
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf16_lossy(slice)
}
