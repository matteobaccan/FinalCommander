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

/// One process in detail: what `F3` on a row of the process panel shows.
///
/// Windows keeps the answers in four different places — the snapshot for the name, the parent
/// and the thread count, `GetProcessMemoryInfo` for the two memory figures,
/// `QueryFullProcessImageNameW` for the executable, `GetProcessTimes` for when it started —
/// and a process of somebody else's, or one of the system's, refuses some of them. Each is
/// asked for on its own and left empty when it is refused, so the card shows what there is
/// rather than nothing at all.
///
/// The command line and the working directory are not here: both live in another process's
/// own memory and are read through `NtQueryInformationProcess`, which is not a documented
/// call with a stable structure behind it. The card leaves those two rows out on Windows
/// rather than carry a page of `unsafe` that a service pack can invalidate.
#[cfg(windows)]
pub fn details(pid: u32) -> Option<super::Details> {
    use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, INVALID_HANDLE_VALUE, MAX_PATH};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows_sys::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };
    use windows_sys::Win32::System::SystemInformation::{GetNativeSystemInfo, SYSTEM_INFO};
    use windows_sys::Win32::System::Threading::{
        GetProcessTimes, IsWow64Process, OpenProcess, QueryFullProcessImageNameW,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };

    // The snapshot first: it is the one place that will say what started this and how many
    // threads it has, and it answers for every process whether or not we may open it.
    // SAFETY: the call takes no pointer of ours and returns a handle or the invalid value.
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return None;
    }
    let mut out: Option<super::Details> = None;
    let mut entry: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
    entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
    // SAFETY: `entry` is a whole, zeroed structure whose `dwSize` says how big it is, which
    // is what the call reads before writing into it.
    let mut ok = unsafe { Process32FirstW(snapshot, &mut entry) };
    while ok != 0 {
        if entry.th32ProcessID == pid {
            let raw = &entry.szExeFile;
            let end = raw.iter().position(|c| *c == 0).unwrap_or(raw.len());
            out = Some(super::Details {
                pid,
                name: String::from_utf16_lossy(&raw[..end]),
                parent: Some(entry.th32ParentProcessID).filter(|p| *p != 0),
                threads: entry.cntThreads,
                ..super::Details::default()
            });
            break;
        }
        // SAFETY: as above; the snapshot is still open.
        ok = unsafe { Process32NextW(snapshot, &mut entry) };
    }
    // SAFETY: the handle came from `CreateToolhelp32Snapshot` and is closed exactly once.
    unsafe { CloseHandle(snapshot) };
    let mut out = out?;

    // SAFETY: no pointer of ours is passed; the result is a handle or null.
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        // Nothing else may be asked about it, which is an answer: the snapshot's three
        // fields are what the card shows.
        return Some(out);
    }

    let mut counters: PROCESS_MEMORY_COUNTERS = unsafe { std::mem::zeroed() };
    counters.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
    // SAFETY: the handle is open and `counters` is a whole structure of the size its own `cb`
    // field declares, which is what the call fills in.
    if unsafe {
        GetProcessMemoryInfo(
            handle,
            &mut counters,
            std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        )
    } != 0
    {
        out.memory = counters.WorkingSetSize as u64;
        // What it has taken from the paging file, which is the figure beside the resident one
        // that Windows will give for nothing.
        out.mapped = counters.PagefileUsage as u64;
    }

    // The executable, with its path. `MAX_PATH` is not enough for a long path, so the buffer
    // is four times it and the length comes back in `len`.
    let mut buffer = vec![0u16; MAX_PATH as usize * 4];
    let mut len = buffer.len() as u32;
    // SAFETY: the handle is open, the buffer is `len` wide, and the call writes at most that
    // many characters into it and sets `len` to how many it wrote.
    if unsafe { QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut len) } != 0 {
        out.exe = Some(String::from_utf16_lossy(&buffer[..len as usize]));
    }

    // 32 or 64: a process running under the emulator is 32-bit, and any other process is
    // whatever the machine itself is.
    let mut wow = 0i32;
    // SAFETY: the handle is open and `wow` is one whole `BOOL` the call writes into.
    if unsafe { IsWow64Process(handle, &mut wow) } != 0 {
        out.bits = if wow != 0 {
            32
        } else {
            let mut info: SYSTEM_INFO = unsafe { std::mem::zeroed() };
            // SAFETY: `info` is a whole, zeroed structure the call fills in; it cannot fail.
            unsafe { GetNativeSystemInfo(&mut info) };
            // `PROCESSOR_ARCHITECTURE_INTEL` is 0 and is the only 32-bit one left.
            // SAFETY: reading one field of the union the call just wrote, as documented.
            let arch = unsafe { info.Anonymous.Anonymous.wProcessorArchitecture };
            if arch == 0 {
                32
            } else {
                64
            }
        };
    }

    let mut started = FILETIME::default();
    let mut exited = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    // SAFETY: the handle is open and all four are whole structures the call writes into.
    if unsafe { GetProcessTimes(handle, &mut started, &mut exited, &mut kernel, &mut user) } != 0 {
        out.started = file_time(started);
    }

    // SAFETY: the handle came from `OpenProcess` and is closed exactly once.
    unsafe { CloseHandle(handle) };
    Some(out)
}

/// Every socket, with the process that holds it (SPEC 02 §7c).
///
/// Windows answers this better than it answers anything else about another process: one call
/// per protocol gives the whole table *and* the owning pid, with no handle to open and
/// nothing to be refused. The four calls are the same shape — ask how big the table is, make
/// room, ask again — and the rows are a variable-length array behind a count, which is why
/// each one is read out by its offset in the buffer rather than through a typed slice.
#[cfg(windows)]
pub fn sockets() -> Vec<super::Socket> {
    use windows_sys::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, NO_ERROR};
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        GetExtendedTcpTable, GetExtendedUdpTable, MIB_TCP6ROW_OWNER_PID, MIB_TCP6TABLE_OWNER_PID,
        MIB_TCPROW_OWNER_PID, MIB_TCPTABLE_OWNER_PID, MIB_UDP6ROW_OWNER_PID,
        MIB_UDP6TABLE_OWNER_PID, MIB_UDPROW_OWNER_PID, MIB_UDPTABLE_OWNER_PID,
        TCP_TABLE_OWNER_PID_ALL, UDP_TABLE_OWNER_PID,
    };

    // The two address families, as `ulAf` wants them: `AF_INET` and `AF_INET6`. They are
    // written out because the constants live behind the whole of the WinSock feature, which
    // is a lot of bindings for two numbers that have not changed since 1995.
    const IPV4: u32 = 2;
    const IPV6: u32 = 23;

    /// The buffer one of the four tables filled, or nothing.
    ///
    /// `ask` is the call, which is handed the buffer and its size and reports how much room
    /// it really needs. Asking twice is the documented way round a table that grows between
    /// the two calls, and a third refusal is taken as "not now".
    fn table(mut ask: impl FnMut(*mut std::ffi::c_void, *mut u32) -> u32) -> Option<Vec<u8>> {
        let mut size = 0u32;
        for _ in 0..3 {
            // SAFETY (in the caller's closure): the first call is given a null pointer and a
            // size of zero, which is how the call is asked for the size it needs.
            let code = ask(std::ptr::null_mut(), &mut size);
            if code != ERROR_INSUFFICIENT_BUFFER {
                return None;
            }
            let mut buffer = vec![0u8; size as usize];
            let code = ask(buffer.as_mut_ptr().cast(), &mut size);
            if code == NO_ERROR {
                return Some(buffer);
            }
            if code != ERROR_INSUFFICIENT_BUFFER {
                return None;
            }
        }
        None
    }

    /// One row out of a table's buffer. The rows follow the count at `offset`, packed, and
    /// the buffer came from a `Vec<u8>` and so is not aligned for the row type: each one is
    /// read unaligned, which is what `read_unaligned` is for.
    fn rows<T: Copy>(buffer: &[u8], offset: usize) -> Vec<T> {
        if buffer.len() < 4 {
            return Vec::new();
        }
        let count = u32::from_ne_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]) as usize;
        let size = std::mem::size_of::<T>();
        let mut out = Vec::with_capacity(count);
        for i in 0..count {
            let at = offset + i * size;
            if at + size > buffer.len() {
                break; // the table shrank under us, which is not worth a panic
            }
            // SAFETY: `at + size` is inside the buffer, the read is unaligned, and `T` is one
            // of the four plain `repr(C)` row structures — all integers and byte arrays, with
            // no padding to be uninitialised and nothing to drop.
            out.push(unsafe { buffer.as_ptr().add(at).cast::<T>().read_unaligned() });
        }
        out
    }

    // A port is kept in the low half of a `DWORD`, in network order.
    let port = |dw: u32| -> u16 { u16::from_be((dw & 0xFFFF) as u16) };
    // An address is kept in network order too, which is the order its bytes are already in.
    let four = |dw: u32| -> String { std::net::Ipv4Addr::from(dw.to_ne_bytes()).to_string() };
    let six = |raw: [u8; 16]| -> String { format!("[{}]", std::net::Ipv6Addr::from(raw)) };

    // The names, once: the pid is in every row and the snapshot is the only place that puts
    // a name to it.
    let names: std::collections::HashMap<u32, String> =
        processes().into_iter().map(|p| (p.pid, p.name)).collect();
    let name_of = |pid: u32| -> (Option<u32>, Option<String>) {
        // Pid zero is the system idle process, which holds nothing worth naming.
        if pid == 0 {
            return (None, None);
        }
        (Some(pid), names.get(&pid).cloned())
    };

    let mut out = Vec::new();

    // TCP over IPv4.
    if let Some(buffer) =
        table(|p, s| unsafe { GetExtendedTcpTable(p, s, 0, IPV4, TCP_TABLE_OWNER_PID_ALL, 0) })
    {
        let offset = std::mem::offset_of!(MIB_TCPTABLE_OWNER_PID, table);
        for row in rows::<MIB_TCPROW_OWNER_PID>(&buffer, offset) {
            let (pid, process) = name_of(row.dwOwningPid);
            let state = win_state(row.dwState);
            out.push(super::Socket {
                proto: "tcp".into(),
                local: format!("{}:{}", four(row.dwLocalAddr), port(row.dwLocalPort)),
                remote: if state == "LISTEN" {
                    String::new()
                } else {
                    format!("{}:{}", four(row.dwRemoteAddr), port(row.dwRemotePort))
                },
                state: state.into(),
                pid,
                process,
            });
        }
    }

    // TCP over IPv6.
    if let Some(buffer) =
        table(|p, s| unsafe { GetExtendedTcpTable(p, s, 0, IPV6, TCP_TABLE_OWNER_PID_ALL, 0) })
    {
        let offset = std::mem::offset_of!(MIB_TCP6TABLE_OWNER_PID, table);
        for row in rows::<MIB_TCP6ROW_OWNER_PID>(&buffer, offset) {
            let (pid, process) = name_of(row.dwOwningPid);
            let state = win_state(row.dwState);
            out.push(super::Socket {
                proto: "tcp6".into(),
                local: format!("{}:{}", six(row.ucLocalAddr), port(row.dwLocalPort)),
                remote: if state == "LISTEN" {
                    String::new()
                } else {
                    format!("{}:{}", six(row.ucRemoteAddr), port(row.dwRemotePort))
                },
                state: state.into(),
                pid,
                process,
            });
        }
    }

    // UDP has no state to be in, and no other end.
    if let Some(buffer) =
        table(|p, s| unsafe { GetExtendedUdpTable(p, s, 0, IPV4, UDP_TABLE_OWNER_PID, 0) })
    {
        let offset = std::mem::offset_of!(MIB_UDPTABLE_OWNER_PID, table);
        for row in rows::<MIB_UDPROW_OWNER_PID>(&buffer, offset) {
            let (pid, process) = name_of(row.dwOwningPid);
            out.push(super::Socket {
                proto: "udp".into(),
                local: format!("{}:{}", four(row.dwLocalAddr), port(row.dwLocalPort)),
                remote: String::new(),
                state: String::new(),
                pid,
                process,
            });
        }
    }
    if let Some(buffer) =
        table(|p, s| unsafe { GetExtendedUdpTable(p, s, 0, IPV6, UDP_TABLE_OWNER_PID, 0) })
    {
        let offset = std::mem::offset_of!(MIB_UDP6TABLE_OWNER_PID, table);
        for row in rows::<MIB_UDP6ROW_OWNER_PID>(&buffer, offset) {
            let (pid, process) = name_of(row.dwOwningPid);
            out.push(super::Socket {
                proto: "udp6".into(),
                local: format!("{}:{}", six(row.ucLocalAddr), port(row.dwLocalPort)),
                remote: String::new(),
                state: String::new(),
                pid,
                process,
            });
        }
    }
    out
}

/// What a drive letter is: the kind of device, its label, its filesystem, and the share behind
/// it where it is a mapped one.
///
/// Three calls, each allowed to fail on its own. An empty CD drive answers the first and
/// refuses the second, which is not an error — it is why the columns are optional.
#[cfg(windows)]
pub fn volume(path: &Path) -> Option<super::Volume> {
    use windows_sys::Win32::Foundation::{ERROR_SUCCESS, MAX_PATH};
    use windows_sys::Win32::NetworkManagement::WNet::WNetGetConnectionW;
    use windows_sys::Win32::Storage::FileSystem::{GetDriveTypeW, GetVolumeInformationW};

    let root = wide(path);
    // SAFETY: `root` is a NUL-terminated wide string that outlives the call, which returns
    // one of the documented `DRIVE_*` numbers and cannot fail otherwise.
    let kind = match unsafe { GetDriveTypeW(root.as_ptr()) } {
        // The numbers `GetDriveTypeW` documents. They are written out because the constants
        // live behind a feature of their own, and they have not moved since Windows 95.
        2 => Some(super::VolumeKind::Removable),
        3 => Some(super::VolumeKind::Fixed),
        4 => Some(super::VolumeKind::Network),
        5 => Some(super::VolumeKind::Optical),
        6 => Some(super::VolumeKind::Memory),
        // 0 is "cannot say" and 1 is "there is no such root": both mean no answer.
        _ => None,
    };

    let mut label = vec![0u16; MAX_PATH as usize + 1];
    let mut fs = vec![0u16; MAX_PATH as usize + 1];
    // SAFETY: both buffers are as long as the counts passed with them, the three outputs we
    // do not want are null, and the call writes at most that many characters into each.
    let described = unsafe {
        GetVolumeInformationW(
            root.as_ptr(),
            label.as_mut_ptr(),
            label.len() as u32,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            fs.as_mut_ptr(),
            fs.len() as u32,
        )
    } != 0;
    let text = |buffer: &[u16]| -> Option<String> {
        let end = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
        let text = String::from_utf16_lossy(&buffer[..end]).trim().to_string();
        (!text.is_empty()).then_some(text)
    };

    // The share behind a mapped letter, which is what the dialog's device column shows. The
    // call wants the letter and the colon and no backslash.
    let mut share = None;
    if kind == Some(super::VolumeKind::Network) {
        let letter: String = path.to_string_lossy().chars().take(2).collect();
        let local = wide_str(&letter);
        let mut buffer = vec![0u16; MAX_PATH as usize * 4];
        let mut len = buffer.len() as u32;
        // SAFETY: `local` is NUL-terminated and outlives the call; the buffer is `len` wide
        // and the call writes at most that many characters into it.
        if unsafe { WNetGetConnectionW(local.as_ptr(), buffer.as_mut_ptr(), &mut len) }
            == ERROR_SUCCESS
        {
            share = text(&buffer);
        }
    }

    Some(super::Volume {
        kind,
        label: described.then(|| text(&label)).flatten(),
        fs: described.then(|| text(&fs)).flatten(),
        share,
        ready: described,
    })
}

/// What is installed, out of the registry key the Control Panel's own list reads.
///
/// Three keys, not one: the machine's, the machine's 32-bit view, and this user's own. A
/// program that is in two of them is the same program, so the first name wins.
///
/// What the Control Panel hides, this hides: an entry with no `DisplayName` is a leftover, one
/// marked `SystemComponent` is a part of Windows rather than a program, and one whose
/// `ReleaseType` says it is an update is an update rather than a program.
#[cfg(windows)]
pub fn programs() -> Vec<super::Program> {
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE,
        KEY_READ,
    };

    const UNINSTALL: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";
    const UNINSTALL32: &str = r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall";

    let mut out: Vec<super::Program> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (root, path) in [
        (HKEY_LOCAL_MACHINE, UNINSTALL),
        (HKEY_LOCAL_MACHINE, UNINSTALL32),
        (HKEY_CURRENT_USER, UNINSTALL),
    ] {
        let mut key: HKEY = std::ptr::null_mut();
        // SAFETY: the path is a NUL-terminated wide string that outlives the call, and `key`
        // is one whole handle the call writes into.
        let opened = unsafe { RegOpenKeyExW(root, wide_str(path).as_ptr(), 0, KEY_READ, &mut key) };
        if opened != ERROR_SUCCESS {
            continue; // a view this build of Windows does not have
        }
        // The subkeys, one program each. A name is at most 255 characters by the registry's
        // own rules, and the call says how many it wrote.
        let mut index = 0u32;
        loop {
            let mut name = [0u16; 256];
            let mut len = name.len() as u32;
            // SAFETY: `name` is `len` wide, the call writes at most that many characters and
            // sets `len` to how many it wrote; every other pointer is null, which the call
            // documents as "do not want".
            let code = unsafe {
                RegEnumKeyExW(
                    key,
                    index,
                    name.as_mut_ptr(),
                    &mut len,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            };
            if code != ERROR_SUCCESS {
                break; // ERROR_NO_MORE_ITEMS, or a key that went away under us
            }
            index += 1;
            let sub = String::from_utf16_lossy(&name[..len as usize]);
            let under = format!("{path}\\{sub}");
            let Some(display) = reg_string(root, &under, "DisplayName") else {
                continue; // a leftover, as the Control Panel treats it
            };
            if display.trim().is_empty() || !seen.insert(display.to_lowercase()) {
                continue;
            }
            if reg_dword(root, &under, "SystemComponent") == Some(1) {
                continue;
            }
            match reg_string(root, &under, "ReleaseType").as_deref() {
                Some("Security Update") | Some("Update") | Some("Hotfix") => continue,
                _ => {}
            }
            out.push(super::Program {
                name: display,
                version: reg_string(root, &under, "DisplayVersion"),
                publisher: reg_string(root, &under, "Publisher"),
                // `EstimatedSize` is in kibibytes, and is an estimate by its own name.
                size: u64::from(reg_dword(root, &under, "EstimatedSize").unwrap_or(0)) * 1024,
                installed: reg_string(root, &under, "InstallDate").map(|d| install_date(&d)),
                location: reg_string(root, &under, "InstallLocation"),
                uninstall: reg_string(root, &under, "QuietUninstallString")
                    .or_else(|| reg_string(root, &under, "UninstallString")),
            });
        }
        // SAFETY: the handle came from `RegOpenKeyExW` and is closed exactly once.
        unsafe { RegCloseKey(key) };
    }
    out
}

/// A string value under a key, or nothing where it is absent, empty or of another type.
#[cfg(windows)]
fn reg_string(
    root: windows_sys::Win32::System::Registry::HKEY,
    path: &str,
    name: &str,
) -> Option<String> {
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::{RegGetValueW, RRF_RT_REG_SZ};

    let (path, name) = (wide_str(path), wide_str(name));
    let mut size = 0u32;
    // SAFETY: both strings are NUL-terminated and outlive the call; a null buffer with a size
    // of zero is how the call is asked how much room it needs.
    let code = unsafe {
        RegGetValueW(
            root,
            path.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut size,
        )
    };
    if code != ERROR_SUCCESS || size == 0 {
        return None;
    }
    // The size is in bytes and the value is wide characters.
    let mut buffer = vec![0u16; (size as usize).div_ceil(2)];
    let mut got = size;
    // SAFETY: the buffer holds `got` bytes, which is what the call was told and what it
    // writes at most.
    let code = unsafe {
        RegGetValueW(
            root,
            path.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            buffer.as_mut_ptr().cast(),
            &mut got,
        )
    };
    if code != ERROR_SUCCESS {
        return None;
    }
    let chars = (got as usize / 2).min(buffer.len());
    let text = String::from_utf16_lossy(&buffer[..chars]);
    let text = text.trim_end_matches('\0').trim().to_string();
    (!text.is_empty()).then_some(text)
}

/// A number value under a key.
#[cfg(windows)]
fn reg_dword(
    root: windows_sys::Win32::System::Registry::HKEY,
    path: &str,
    name: &str,
) -> Option<u32> {
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::{RegGetValueW, RRF_RT_REG_DWORD};

    let (path, name) = (wide_str(path), wide_str(name));
    let mut value = 0u32;
    let mut size = std::mem::size_of::<u32>() as u32;
    // SAFETY: both strings are NUL-terminated and outlive the call, and `value` is one whole
    // `u32` of exactly the size the call is told.
    let code = unsafe {
        RegGetValueW(
            root,
            path.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            std::ptr::from_mut(&mut value).cast(),
            &mut size,
        )
    };
    (code == ERROR_SUCCESS).then_some(value)
}

/// `20240317` as `2024-03-17`, and anything else as it was written.
#[cfg(windows)]
fn install_date(raw: &str) -> String {
    if raw.len() == 8 && raw.chars().all(|c| c.is_ascii_digit()) {
        format!("{}-{}-{}", &raw[..4], &raw[4..6], &raw[6..])
    } else {
        raw.to_string()
    }
}

/// A Rust string as Windows wants one: UTF-16 with a NUL after it. (`wide`, above, is the
/// same for a path, which cannot simply be borrowed as a `str` on Windows.)
#[cfg(windows)]
fn wide_str(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// The twelve states `MIB_TCP_STATE` has, which are numbered differently from the ones Linux
/// writes in its own table — hence a second list rather than a shared one.
#[cfg(windows)]
fn win_state(code: u32) -> &'static str {
    match code {
        1 => "CLOSED",
        2 => "LISTEN",
        3 => "SYN_SENT",
        4 => "SYN_RECV",
        5 => "ESTABLISHED",
        6 => "FIN_WAIT1",
        7 => "FIN_WAIT2",
        8 => "CLOSE_WAIT",
        9 => "CLOSING",
        10 => "LAST_ACK",
        11 => "TIME_WAIT",
        12 => "DELETE_TCB",
        _ => "",
    }
}

/// A Windows `FILETIME` — hundreds of nanoseconds since the first of January 1601 — as a
/// `SystemTime`. The difference between the two epochs is 11,644,473,600 seconds, and a time
/// before the Unix one is not a time a process started.
#[cfg(windows)]
fn file_time(ft: windows_sys::Win32::Foundation::FILETIME) -> Option<std::time::SystemTime> {
    use std::time::{Duration, SystemTime};

    let ticks = (u64::from(ft.dwHighDateTime) << 32) | u64::from(ft.dwLowDateTime);
    let secs = ticks / 10_000_000;
    let nanos = ((ticks % 10_000_000) * 100) as u32;
    let unix = secs.checked_sub(11_644_473_600)?;
    Some(SystemTime::UNIX_EPOCH + Duration::new(unix, nanos))
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
