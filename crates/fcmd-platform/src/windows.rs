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
    // The kernel's own list gives the resident size and the start of every process, ours or
    // not; asking each process in turn is the fallback where it will not answer.
    let nt = nt_processes().unwrap_or_default();
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
        let known = nt.get(&entry.th32ProcessID);
        out.push(super::Process {
            pid: entry.th32ProcessID,
            name,
            memory: known
                .map(|k| k.resident)
                .filter(|m| *m > 0)
                .unwrap_or_else(|| memory_of(entry.th32ProcessID)),
            threads: entry.cntThreads,
            bits: 0,
            started: known.and_then(|k| k.started),
        });
        // SAFETY: as above; the snapshot is still open.
        ok = unsafe { Process32NextW(snapshot, &mut entry) };
    }
    // SAFETY: the handle came from `CreateToolhelp32Snapshot` and is closed exactly once.
    unsafe { CloseHandle(snapshot) };
    out
}

/// End a process, as Task Manager's *End task* does.
#[cfg(windows)]
pub fn kill(pid: u32) -> Result<(), String> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
    // SAFETY: no pointer of ours is passed; the result is a handle or null.
    let handle = unsafe { OpenProcess(PROCESS_TERMINATE, 0, pid) };
    if handle.is_null() {
        return Err(std::io::Error::last_os_error().to_string());
    }
    // SAFETY: the handle is open with the right to end the process, and is closed once.
    let done = unsafe { TerminateProcess(handle, 1) };
    let error = std::io::Error::last_os_error();
    unsafe { CloseHandle(handle) };
    if done == 0 {
        return Err(error.to_string());
    }
    Ok(())
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
        EnumProcessModules, GetModuleFileNameExW, GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
        PROCESS_MEMORY_COUNTERS_EX,
    };
    use windows_sys::Win32::System::SystemInformation::{GetNativeSystemInfo, SYSTEM_INFO};
    use windows_sys::Win32::System::Threading::{
        GetPriorityClass, GetProcessHandleCount, GetProcessIoCounters, GetProcessTimes,
        IsWow64Process, OpenProcess, QueryFullProcessImageNameW, IO_COUNTERS,
        PROCESS_QUERY_INFORMATION, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ,
    };
    // The one answer on the card that is not in the process API: a session is a Terminal
    // Services idea, and that is where Windows keeps it.
    use windows_sys::Win32::System::RemoteDesktop::ProcessIdToSessionId;

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

    // The kernel's counters, which it gives for every process without our opening it: what
    // FAR's card shows for a service as well as for a program of ours. What the process
    // itself answers below, when we may ask it, is written over them.
    let base_priority = nt_processes()
        .and_then(|mut all| all.remove(&pid))
        .map(|k| {
            out.threads = k.threads.max(out.threads);
            out.started = k.started;
            out.cpu_user = Some(k.user);
            out.cpu_kernel = Some(k.kernel);
            out.handles = k.handles;
            out.session = Some(k.session);
            out.memory = k.resident;
            out.memory_peak = k.resident_peak;
            out.virtual_size = k.virtual_size;
            out.virtual_peak = k.virtual_peak;
            out.paged_pool = k.paged_pool;
            out.nonpaged_pool = k.nonpaged_pool;
            out.mapped = k.pagefile;
            out.swap = k.pagefile;
            out.swap_peak = k.pagefile_peak;
            out.private = k.private;
            out.private_resident = k.private_resident;
            out.faults = k.faults;
            out.io_read = k.read_bytes;
            out.io_write = k.write_bytes;
            out.io_other = k.other_bytes;
            out.io_read_ops = k.read_ops;
            out.io_write_ops = k.write_ops;
            out.io_other_ops = k.other_ops;
            out.priority = Some(k.base_priority.to_string());
            k.base_priority
        });

    // SAFETY: no pointer of ours is passed; the result is a handle or null.
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        // Nothing else may be asked about it, which is an answer: the snapshot's three
        // fields are what the card shows.
        return Some(out);
    }

    // The extended counters: the same structure with the private bytes on the end, which is
    // the figure Task Manager shows and the one worth having beside the resident size. The
    // call is told how big the structure is, so an older system fills in the part it knows.
    let mut counters: PROCESS_MEMORY_COUNTERS_EX = unsafe { std::mem::zeroed() };
    counters.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
    // SAFETY: the handle is open and `counters` is a whole structure of the size its own `cb`
    // field declares, which is what the call fills in. The extended structure begins with the
    // plain one, which is why the pointer may be handed over as that.
    if unsafe {
        GetProcessMemoryInfo(
            handle,
            (&mut counters as *mut PROCESS_MEMORY_COUNTERS_EX).cast::<PROCESS_MEMORY_COUNTERS>(),
            std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32,
        )
    } != 0
    {
        out.memory = counters.WorkingSetSize as u64;
        // What it has taken from the paging file, which is the figure beside the resident one
        // that Windows will give for nothing.
        out.mapped = counters.PagefileUsage as u64;
        out.private = counters.PrivateUsage as u64;
        out.swap = counters.PagefileUsage as u64;
        out.faults = counters.PageFaultCount as u64;
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
        // The two spans are counted in the same hundred-nanosecond units the timestamps are,
        // but from zero rather than from 1601: they are durations, not dates.
        out.cpu_kernel = Some(span(kernel));
        out.cpu_user = Some(span(user));
    }

    // How many handles of every kind it holds, which is the figure that answers "why will
    // this file not close".
    let mut handles = 0u32;
    // SAFETY: the handle is open and `handles` is one whole `u32` the call writes into.
    if unsafe { GetProcessHandleCount(handle, &mut handles) } != 0 {
        out.handles = handles;
    }

    // What it has read and written, as Windows has counted it.
    let mut io: IO_COUNTERS = unsafe { std::mem::zeroed() };
    // SAFETY: the handle is open and `io` is a whole, zeroed structure the call fills in.
    if unsafe { GetProcessIoCounters(handle, &mut io) } != 0 {
        out.io_read = io.ReadTransferCount;
        out.io_write = io.WriteTransferCount;
    }

    // How it is scheduled. The call answers with the class, and zero means it refused.
    // SAFETY: the handle is open; the call takes nothing else.
    let class = unsafe { GetPriorityClass(handle) };
    if class != 0 {
        out.priority = Some(match base_priority {
            Some(base) => format!("{} ({base})", priority_name(class)),
            None => priority_name(class).to_string(),
        });
    }

    // Which login session it belongs to: service or desktop, and whose desktop.
    let mut session = 0u32;
    // SAFETY: `session` is one whole `u32` the call writes into; the pid is a number.
    if unsafe { ProcessIdToSessionId(pid, &mut session) } != 0 {
        out.session = Some(session);
    }

    // Every library it has loaded. Reading another process's module list wants more rights
    // than the rest of this card does — it reads that process's memory — so it is a second
    // handle, and a process that refuses it simply has no module list on its card.
    // SAFETY: no pointer of ours is passed; the result is a handle or null.
    let vm = unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, pid) };
    if !vm.is_null() {
        const SLOT: usize = std::mem::size_of::<windows_sys::Win32::Foundation::HMODULE>();
        let mut modules: Vec<windows_sys::Win32::Foundation::HMODULE> =
            vec![std::ptr::null_mut(); 512];
        let mut needed = 0u32;
        for _ in 0..2 {
            let room = (modules.len() * SLOT) as u32;
            // SAFETY: the handle is open and the array is `room` bytes long, which is what
            // the call is told; it writes at most that and sets `needed` to what it wanted.
            if unsafe { EnumProcessModules(vm, modules.as_mut_ptr(), room, &mut needed) } == 0 {
                needed = 0;
                break;
            }
            if needed <= room {
                break;
            }
            modules.resize(needed as usize / SLOT + 1, std::ptr::null_mut());
        }
        let count = (needed as usize / SLOT).min(modules.len());
        for module in modules.iter().take(count) {
            let mut buffer = vec![0u16; MAX_PATH as usize * 4];
            // SAFETY: the handle is open, the module came from `EnumProcessModules` on it,
            // and the buffer is as long as the call is told it is.
            let len = unsafe {
                GetModuleFileNameExW(vm, *module, buffer.as_mut_ptr(), buffer.len() as u32)
            };
            if len > 0 {
                out.modules
                    .push(String::from_utf16_lossy(&buffer[..len as usize]));
            }
        }
        // SAFETY: the handle came from `OpenProcess` and is closed exactly once.
        unsafe { CloseHandle(vm) };
    }

    // Whose it is, and the SID behind the name: the pair FAR's own card shows.
    if let Some((user, sid)) = token_owner(handle) {
        out.user = Some(user);
        out.sid = Some(sid);
    }

    // SAFETY: the handle came from `OpenProcess` and is closed exactly once.
    unsafe { CloseHandle(handle) };
    Some(out)
}

/// A `FILETIME` used as a span rather than as a date: hundred-nanosecond units from zero.
#[cfg(windows)]
fn span(time: windows_sys::Win32::Foundation::FILETIME) -> std::time::Duration {
    let units = ((time.dwHighDateTime as u64) << 32) | time.dwLowDateTime as u64;
    std::time::Duration::from_nanos(units * 100)
}

/// What a priority class is called, in the words the system's own tools use.
#[cfg(windows)]
fn priority_name(class: u32) -> &'static str {
    use windows_sys::Win32::System::Threading::{
        ABOVE_NORMAL_PRIORITY_CLASS, BELOW_NORMAL_PRIORITY_CLASS, HIGH_PRIORITY_CLASS,
        IDLE_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS, REALTIME_PRIORITY_CLASS,
    };
    match class {
        IDLE_PRIORITY_CLASS => "Idle",
        BELOW_NORMAL_PRIORITY_CLASS => "Below normal",
        NORMAL_PRIORITY_CLASS => "Normal",
        ABOVE_NORMAL_PRIORITY_CLASS => "Above normal",
        HIGH_PRIORITY_CLASS => "High",
        REALTIME_PRIORITY_CLASS => "Realtime",
        _ => "Unknown",
    }
}

/// Whose a process is: the account name, and the SID it stands for.
///
/// Windows keeps the owner in the process's own token rather than beside the process, so it
/// is three calls: open the token, ask it how big its user record is, ask again with room —
/// and then the SID is turned into a name and into the `S-1-5-…` string FAR's card shows.
#[cfg(windows)]
fn token_owner(process: windows_sys::Win32::Foundation::HANDLE) -> Option<(String, String)> {
    use windows_sys::Win32::Foundation::{CloseHandle, LocalFree};
    use windows_sys::Win32::Security::Authorization::ConvertSidToStringSidW;
    use windows_sys::Win32::Security::{GetTokenInformation, TokenUser, TOKEN_QUERY, TOKEN_USER};
    use windows_sys::Win32::System::Threading::OpenProcessToken;

    let mut token = std::ptr::null_mut();
    // SAFETY: the process handle is open and `token` is one whole handle the call writes.
    if unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) } == 0 {
        return None;
    }
    let mut needed = 0u32;
    // SAFETY: asking with no buffer is how the call is asked how much room it wants; it
    // fails with "insufficient buffer" and sets `needed`, which is the documented shape.
    unsafe { GetTokenInformation(token, TokenUser, std::ptr::null_mut(), 0, &mut needed) };
    let mut buffer = vec![0u8; needed.max(1) as usize];
    // SAFETY: the token is open and the buffer is `needed` bytes long, which is what the
    // call is told and what it asked for a moment ago.
    let ok = unsafe {
        GetTokenInformation(
            token,
            TokenUser,
            buffer.as_mut_ptr().cast(),
            needed,
            &mut needed,
        )
    };
    // SAFETY: the token came from `OpenProcessToken` and is closed exactly once.
    unsafe { CloseHandle(token) };
    if ok == 0 || buffer.len() < std::mem::size_of::<TOKEN_USER>() {
        return None;
    }
    // SAFETY: the call filled the buffer with a `TOKEN_USER` whose `Sid` points inside it,
    // and the buffer outlives both of the uses below. It is read unaligned because it came
    // from a `Vec<u8>`.
    let sid = unsafe { std::ptr::read_unaligned(buffer.as_ptr().cast::<TOKEN_USER>()) }
        .User
        .Sid;
    if sid.is_null() {
        return None;
    }

    // The SID as the `S-1-5-...` text FAR's own card shows beside the name.
    let mut text: *mut u16 = std::ptr::null_mut();
    // SAFETY: the SID is the one the token gave us and `text` is one whole pointer the call
    // writes; what it allocates is freed below with `LocalFree`, as documented.
    let as_string = if unsafe { ConvertSidToStringSidW(sid, &mut text) } != 0 && !text.is_null() {
        // SAFETY: the call wrote a NUL-terminated UTF-16 string at `text`.
        let out = unsafe { from_wide(text) };
        // SAFETY: `text` came from the call's own allocator, and is freed exactly once.
        unsafe { LocalFree(text.cast()) };
        out
    } else {
        String::new()
    };
    // The name behind it, through the same lookup a directory listing uses. A cache is of no
    // use for one process, but that function is the one place that knows how to ask.
    let mut cache = super::OwnerCache::default();
    let name = account_name(sid, &mut cache).unwrap_or_else(|| as_string.clone());
    if name.is_empty() && as_string.is_empty() {
        return None;
    }
    Some((name, as_string))
}

/// Everything the machine runs in the background, from the Service Control Manager.
///
/// One call gives the whole list with each service's state and its process: the manager is
/// asked how much room the list needs, given that much, and asked again. Every user may read
/// it; only an administrator may change anything, which is what `control_service` finds out.
#[cfg(windows)]
pub fn services() -> Vec<super::Service> {
    use windows_sys::Win32::System::Services::{
        CloseServiceHandle, EnumServicesStatusExW, OpenSCManagerW, ENUM_SERVICE_STATUS_PROCESSW,
        SC_ENUM_PROCESS_INFO, SC_MANAGER_CONNECT, SC_MANAGER_ENUMERATE_SERVICE, SERVICE_RUNNING,
        SERVICE_STATE_ALL, SERVICE_STOPPED, SERVICE_WIN32,
    };

    // SAFETY: both names are null, which asks for this machine and the active database, as
    // documented; the result is a handle or null.
    let scm = unsafe {
        OpenSCManagerW(
            std::ptr::null(),
            std::ptr::null(),
            SC_MANAGER_CONNECT | SC_MANAGER_ENUMERATE_SERVICE,
        )
    };
    if scm.is_null() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut buffer: Vec<u8> = Vec::new();
    let mut needed = 0u32;
    let mut count = 0u32;
    let mut resume = 0u32;
    for _ in 0..2 {
        // SAFETY: the buffer is as long as the call is told it is — zero on the first pass,
        // which is how it is asked how much room it wants — and the four out parameters are
        // whole values of ours.
        let ok = unsafe {
            EnumServicesStatusExW(
                scm,
                SC_ENUM_PROCESS_INFO,
                SERVICE_WIN32,
                SERVICE_STATE_ALL,
                buffer.as_mut_ptr(),
                buffer.len() as u32,
                &mut needed,
                &mut count,
                &mut resume,
                std::ptr::null(),
            )
        };
        if ok != 0 {
            break;
        }
        if needed == 0 {
            // SAFETY: the handle came from `OpenSCManagerW` and is closed exactly once.
            unsafe { CloseServiceHandle(scm) };
            return Vec::new();
        }
        buffer = vec![0u8; needed as usize];
        count = 0;
        resume = 0;
    }
    let stride = std::mem::size_of::<ENUM_SERVICE_STATUS_PROCESSW>();
    for i in 0..count as usize {
        let at = i * stride;
        if at + stride > buffer.len() {
            break;
        }
        // SAFETY: the manager filled the buffer with `count` of these, packed; the buffer is
        // a `Vec<u8>` and so is not aligned for the type, which is what `read_unaligned` is
        // for. The two strings point into the same buffer and are read before it is dropped.
        let row = unsafe {
            std::ptr::read_unaligned(
                buffer
                    .as_ptr()
                    .add(at)
                    .cast::<ENUM_SERVICE_STATUS_PROCESSW>(),
            )
        };
        let name = if row.lpServiceName.is_null() {
            String::new()
        } else {
            // SAFETY: the manager wrote a NUL-terminated string there.
            unsafe { from_wide(row.lpServiceName) }
        };
        if name.is_empty() {
            continue;
        }
        let display = if row.lpDisplayName.is_null() {
            String::new()
        } else {
            // SAFETY: as above.
            unsafe { from_wide(row.lpDisplayName) }
        };
        let status = row.ServiceStatusProcess;
        out.push(super::Service {
            id: name,
            display,
            state: match status.dwCurrentState {
                SERVICE_RUNNING => super::ServiceState::Running,
                SERVICE_STOPPED => super::ServiceState::Stopped,
                _ => super::ServiceState::Other,
            },
            description: String::new(),
            pid: Some(status.dwProcessId).filter(|p| *p != 0),
        });
    }
    // SAFETY: the handle came from `OpenSCManagerW` and is closed exactly once.
    unsafe { CloseServiceHandle(scm) };
    out
}

/// Start, stop or restart one service. A restart is a stop, a wait for it to be down, and a
/// start: the manager has no single call for it.
#[cfg(windows)]
pub fn control_service(id: &str, action: super::ServiceAction) -> Result<(), super::ServiceError> {
    use windows_sys::Win32::System::Services::{
        CloseServiceHandle, ControlService, OpenSCManagerW, OpenServiceW, QueryServiceStatus,
        StartServiceW, SC_MANAGER_CONNECT, SERVICE_CONTROL_STOP, SERVICE_QUERY_STATUS,
        SERVICE_START, SERVICE_STATUS, SERVICE_STOP, SERVICE_STOPPED,
    };

    let wide: Vec<u16> = id.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: both names are null, as above; the result is a handle or null.
    let scm = unsafe { OpenSCManagerW(std::ptr::null(), std::ptr::null(), SC_MANAGER_CONNECT) };
    if scm.is_null() {
        return Err(last_error_as_service_error());
    }
    // SAFETY: the handle is open and the name is a NUL-terminated string of ours.
    let service = unsafe {
        OpenServiceW(
            scm,
            wide.as_ptr(),
            SERVICE_START | SERVICE_STOP | SERVICE_QUERY_STATUS,
        )
    };
    if service.is_null() {
        let problem = last_error_as_service_error();
        // SAFETY: the manager handle came from `OpenSCManagerW` and is closed exactly once.
        unsafe { CloseServiceHandle(scm) };
        return Err(problem);
    }

    let stop = |status: &mut SERVICE_STATUS| -> bool {
        // SAFETY: the service handle is open and `status` is a whole structure the call fills.
        unsafe { ControlService(service, SERVICE_CONTROL_STOP, status) != 0 }
    };
    let mut status: SERVICE_STATUS = unsafe { std::mem::zeroed() };
    let outcome = match action {
        // SAFETY: the handle is open; no argument vector is passed, which is the documented
        // way of starting a service with the arguments it was configured with.
        super::ServiceAction::Start => unsafe { StartServiceW(service, 0, std::ptr::null()) != 0 },
        super::ServiceAction::Stop => stop(&mut status),
        super::ServiceAction::Restart => {
            // SAFETY: the handle is open and `status` is a whole structure of ours.
            let asked = unsafe { QueryServiceStatus(service, &mut status) } != 0;
            let down = stop(&mut status) || (asked && status.dwCurrentState == SERVICE_STOPPED);
            if down {
                // The manager stops a service in its own time, so it is asked how it is
                // getting on rather than started again at once: a `StartService` on a service
                // that is still stopping is refused.
                for _ in 0..50 {
                    // SAFETY: as above.
                    if unsafe { QueryServiceStatus(service, &mut status) } == 0 {
                        break;
                    }
                    if status.dwCurrentState == SERVICE_STOPPED {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
            // SAFETY: the handle is open; see `Start` above.
            unsafe { StartServiceW(service, 0, std::ptr::null()) != 0 }
        }
    };
    let problem = (!outcome).then(last_error_as_service_error);
    // SAFETY: both handles came from the calls above and are closed exactly once each.
    unsafe {
        CloseServiceHandle(service);
        CloseServiceHandle(scm);
    }
    match problem {
        Some(problem) => Err(problem),
        None => Ok(()),
    }
}

/// What the last call went wrong with, in the two kinds the caller cares about.
#[cfg(windows)]
fn last_error_as_service_error() -> super::ServiceError {
    use windows_sys::Win32::Foundation::{GetLastError, ERROR_ACCESS_DENIED};

    // SAFETY: the call takes nothing and reads this thread's own last error.
    let code = unsafe { GetLastError() };
    if code == ERROR_ACCESS_DENIED {
        super::ServiceError::NeedsRoot
    } else {
        super::ServiceError::Failed(format!("error {code}"))
    }
}

/// Every socket, with the process that holds it (SPEC 02 §14).
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
pub fn drive_kind(path: &Path) -> Option<super::VolumeKind> {
    use windows_sys::Win32::Storage::FileSystem::GetDriveTypeW;
    let root = wide(path);
    // SAFETY: `root` is a NUL-terminated wide string that outlives the call.
    match unsafe { GetDriveTypeW(root.as_ptr()) } {
        2 => Some(super::VolumeKind::Removable),
        3 => Some(super::VolumeKind::Fixed),
        4 => Some(super::VolumeKind::Network),
        5 => Some(super::VolumeKind::Optical),
        6 => Some(super::VolumeKind::Memory),
        _ => None,
    }
}

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
/// What the kernel keeps about one process, read for every process at once.
#[cfg(windows)]
#[derive(Debug, Clone, Default)]
struct NtProcess {
    threads: u32,
    started: Option<std::time::SystemTime>,
    user: std::time::Duration,
    kernel: std::time::Duration,
    base_priority: i32,
    handles: u32,
    session: u32,
    virtual_peak: u64,
    virtual_size: u64,
    faults: u64,
    resident_peak: u64,
    resident: u64,
    paged_pool: u64,
    nonpaged_pool: u64,
    pagefile: u64,
    pagefile_peak: u64,
    private: u64,
    private_resident: u64,
    read_ops: u64,
    write_ops: u64,
    other_ops: u64,
    read_bytes: u64,
    write_bytes: u64,
    other_bytes: u64,
}

/// `SYSTEM_PROCESS_INFORMATION`, as the kernel lays it out (the part before the threads).
#[cfg(windows)]
#[repr(C)]
#[allow(dead_code)]
struct SystemProcessInformation {
    next_entry_offset: u32,
    number_of_threads: u32,
    working_set_private_size: i64,
    hard_fault_count: u32,
    number_of_threads_high_watermark: u32,
    cycle_time: u64,
    create_time: i64,
    user_time: i64,
    kernel_time: i64,
    image_name_length: u16,
    image_name_maximum_length: u16,
    image_name_buffer: *mut u16,
    base_priority: i32,
    unique_process_id: usize,
    inherited_from_unique_process_id: usize,
    handle_count: u32,
    session_id: u32,
    unique_process_key: usize,
    peak_virtual_size: usize,
    virtual_size: usize,
    page_fault_count: u32,
    peak_working_set_size: usize,
    working_set_size: usize,
    quota_peak_paged_pool_usage: usize,
    quota_paged_pool_usage: usize,
    quota_peak_non_paged_pool_usage: usize,
    quota_non_paged_pool_usage: usize,
    pagefile_usage: usize,
    peak_pagefile_usage: usize,
    private_page_count: usize,
    read_operation_count: i64,
    write_operation_count: i64,
    other_operation_count: i64,
    read_transfer_count: i64,
    write_transfer_count: i64,
    other_transfer_count: i64,
}

/// Every process the kernel knows, with the counters FAR's process list shows, read in one
/// call that needs no right over any of them — which is why a service running as SYSTEM has
/// its memory and its times here when opening it is refused. `None` if the call is not
/// there or will not answer.
#[cfg(windows)]
fn nt_processes() -> Option<std::collections::HashMap<u32, NtProcess>> {
    use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
    type Query = unsafe extern "system" fn(u32, *mut core::ffi::c_void, u32, *mut u32) -> i32;
    const SYSTEM_PROCESS_INFORMATION: u32 = 5;
    const STATUS_INFO_LENGTH_MISMATCH: i32 = 0xC000_0004_u32 as i32;

    let ntdll: Vec<u16> = "ntdll.dll\0".encode_utf16().collect();
    // SAFETY: ntdll is loaded in every process; the name is NUL-terminated; the address is
    // turned into the signature the function has had since Windows NT.
    let query: Query = unsafe {
        let module = GetModuleHandleW(ntdll.as_ptr());
        if module.is_null() {
            return None;
        }
        let address = GetProcAddress(module, c"NtQuerySystemInformation".as_ptr().cast())?;
        std::mem::transmute::<unsafe extern "system" fn() -> isize, Query>(address)
    };
    // `u64` so the buffer is aligned for the structure; it grows until the list fits.
    let mut buffer: Vec<u64> = vec![0; 64 * 1024];
    let mut status = STATUS_INFO_LENGTH_MISMATCH;
    for _ in 0..8 {
        let mut needed = 0u32;
        let bytes = (buffer.len() * 8) as u32;
        // SAFETY: the buffer is `bytes` long and writable; `needed` is one `u32`.
        status = unsafe {
            query(
                SYSTEM_PROCESS_INFORMATION,
                buffer.as_mut_ptr().cast(),
                bytes,
                &mut needed,
            )
        };
        if status != STATUS_INFO_LENGTH_MISMATCH {
            break;
        }
        let want = (needed as usize).max(bytes as usize * 2) / 8 + 1024;
        buffer = vec![0; want];
    }
    if status < 0 {
        return None;
    }
    let base = buffer.as_ptr().cast::<u8>();
    let end = buffer.len() * 8;
    let mut out = std::collections::HashMap::new();
    let mut offset = 0usize;
    let span = |t: i64| std::time::Duration::from_nanos((t.max(0) as u64) * 100);
    loop {
        if offset + std::mem::size_of::<SystemProcessInformation>() > end {
            break;
        }
        // SAFETY: the entry lies inside the buffer the kernel filled, at an offset the kernel
        // gave; it is read unaligned so no assumption is made about the offset.
        let p = unsafe {
            std::ptr::read_unaligned(base.add(offset).cast::<SystemProcessInformation>())
        };
        let created = windows_sys::Win32::Foundation::FILETIME {
            dwLowDateTime: p.create_time as u32,
            dwHighDateTime: (p.create_time as u64 >> 32) as u32,
        };
        out.insert(
            p.unique_process_id as u32,
            NtProcess {
                threads: p.number_of_threads,
                started: (p.create_time > 0).then(|| file_time(created)).flatten(),
                user: span(p.user_time),
                kernel: span(p.kernel_time),
                base_priority: p.base_priority,
                handles: p.handle_count,
                session: p.session_id,
                virtual_peak: p.peak_virtual_size as u64,
                virtual_size: p.virtual_size as u64,
                faults: u64::from(p.page_fault_count),
                resident_peak: p.peak_working_set_size as u64,
                resident: p.working_set_size as u64,
                paged_pool: p.quota_paged_pool_usage as u64,
                nonpaged_pool: p.quota_non_paged_pool_usage as u64,
                pagefile: p.pagefile_usage as u64,
                pagefile_peak: p.peak_pagefile_usage as u64,
                private: p.private_page_count as u64,
                private_resident: p.working_set_private_size.max(0) as u64,
                read_ops: p.read_operation_count.max(0) as u64,
                write_ops: p.write_operation_count.max(0) as u64,
                other_ops: p.other_operation_count.max(0) as u64,
                read_bytes: p.read_transfer_count.max(0) as u64,
                write_bytes: p.write_transfer_count.max(0) as u64,
                other_bytes: p.other_transfer_count.max(0) as u64,
            },
        );
        if p.next_entry_offset == 0 {
            break;
        }
        offset += p.next_entry_offset as usize;
    }
    Some(out)
}

/// Shift, Ctrl and Alt as the keyboard has them now, while the console is in front: its own
/// window in the old console host, or the terminal window that owns the pseudo-console
/// window in Windows Terminal.
pub fn held_keys() -> Option<super::HeldKeys> {
    use windows_sys::Win32::System::Console::GetConsoleWindow;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, VK_CONTROL, VK_MENU, VK_SHIFT,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetAncestor, GetForegroundWindow, GetWindow, GA_ROOTOWNER, GW_OWNER,
    };
    // SAFETY: none of these take pointers or keep anything; a null window is a valid answer
    // and is checked before it is compared.
    let in_front = unsafe {
        let console = GetConsoleWindow();
        let front = GetForegroundWindow();
        !console.is_null()
            && !front.is_null()
            && (front == console
                || GetWindow(console, GW_OWNER) == front
                || GetAncestor(console, GA_ROOTOWNER) == front)
    };
    if !in_front {
        return None;
    }
    // The high bit is "down now".
    let down = |key: u16| unsafe { GetAsyncKeyState(i32::from(key)) } < 0;
    Some(super::HeldKeys {
        shift: down(VK_SHIFT),
        ctrl: down(VK_CONTROL),
        alt: down(VK_MENU),
    })
}

/// The keypad's `Clear`, read off the console before the terminal library sees it (issue 45).
///
/// The library reads key records and keeps the ones that carry a character or have a name;
/// `VK_CLEAR` has neither, so it is lost there. So the first record is looked at — peeked, not
/// read — and taken only when it is that key going down. Everything else is left where it
/// is, in its place in the queue, for the library to read as it always has.
pub fn console_key(wait: std::time::Duration) -> super::ConsoleKey {
    use super::ConsoleKey;
    use windows_sys::Win32::Foundation::{INVALID_HANDLE_VALUE, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Console::{
        GetConsoleMode, GetStdHandle, PeekConsoleInputW, ReadConsoleInputW, INPUT_RECORD,
        KEY_EVENT, LEFT_ALT_PRESSED, LEFT_CTRL_PRESSED, RIGHT_ALT_PRESSED, RIGHT_CTRL_PRESSED,
        SHIFT_PRESSED, STD_INPUT_HANDLE,
    };
    use windows_sys::Win32::System::Threading::WaitForSingleObject;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_CLEAR;

    let ms = u32::try_from(wait.as_millis()).unwrap_or(u32::MAX);
    // SAFETY: the handle is the process's own standard input, checked before use; the record
    // is a plain struct the calls fill, one at a time, and the counts are read only after the
    // call that sets them has said it succeeded.
    unsafe {
        // Input that is not a console — redirected from a file or a pipe — is always
        // "ready" to a wait, and would turn the reader's loop into a spin: there, the time
        // is waited out and the library left to read as it does anyway.
        let input = GetStdHandle(STD_INPUT_HANDLE);
        let mut mode = 0u32;
        if input.is_null() || input == INVALID_HANDLE_VALUE || GetConsoleMode(input, &mut mode) == 0
        {
            std::thread::sleep(wait);
            return ConsoleKey::Other;
        }
        if WaitForSingleObject(input, ms) != WAIT_OBJECT_0 {
            return ConsoleKey::Nothing;
        }
        let mut record: INPUT_RECORD = std::mem::zeroed();
        let mut count = 0u32;
        if PeekConsoleInputW(input, &mut record, 1, &mut count) == 0 || count == 0 {
            return ConsoleKey::Nothing;
        }
        if u32::from(record.EventType) != KEY_EVENT {
            return ConsoleKey::Other;
        }
        let key = record.Event.KeyEvent;
        if key.bKeyDown == 0 || key.wVirtualKeyCode != VK_CLEAR {
            return ConsoleKey::Other;
        }
        let mut taken = 0u32;
        if ReadConsoleInputW(input, &mut record, 1, &mut taken) == 0 || taken == 0 {
            return ConsoleKey::Other;
        }
        let state = key.dwControlKeyState;
        ConsoleKey::Clear(super::HeldKeys {
            shift: state & SHIFT_PRESSED != 0,
            ctrl: state & (LEFT_CTRL_PRESSED | RIGHT_CTRL_PRESSED) != 0,
            alt: state & (LEFT_ALT_PRESSED | RIGHT_ALT_PRESSED) != 0,
        })
    }
}
