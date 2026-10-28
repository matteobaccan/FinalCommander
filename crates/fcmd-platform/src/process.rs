//! What is running on this machine (SPEC 02 §7b).
//!
//! FAR has a process panel and it earns its place: when a file will not be deleted or a
//! device will not unmount, what one wants is the list of what is running, sorted by what is
//! taking the memory, without leaving the program. Every platform keeps the list somewhere
//! different — a directory of numbers on Linux, a snapshot handle on Windows, a `sysctl` on
//! macOS — and this is the one place that knows which.
//!
//! Everything but the pid is best-effort. A process may end between being listed and being
//! asked about, and a process that belongs to somebody else may refuse to answer at all; a
//! field nobody would tell us is left empty rather than filled with a guess.

/// One running process, as the panel shows it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Process {
    pub pid: u32,
    /// The executable's own name — `chrome.exe`, `firefox` — and not its whole path: that is
    /// what a column of a panel has room for.
    pub name: String,
    /// Resident memory, in bytes. Zero where nothing would say.
    pub memory: u64,
    /// How many threads it has, or zero where nothing would say.
    pub threads: u32,
    /// 32 or 64, for the bitness column, or zero where nothing would say.
    pub bits: u8,
}

/// Everything running, as far as this user is allowed to see.
///
/// The order is whatever the system gives; the panel sorts it, as it sorts a directory.
pub fn processes() -> Vec<Process> {
    #[cfg(target_os = "linux")]
    {
        linux()
    }
    #[cfg(windows)]
    {
        super::windows::processes()
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        // macOS and the BSDs keep the list behind a `sysctl` whose shape is a C structure
        // this crate would have to mirror by hand, field for field, per release. That is a
        // page of `unsafe` for a panel, and a wrong offset there reads somebody else's
        // memory: it waits until there is a tested way to do it (`docs/BACKLOG.md`).
        Vec::new()
    }
}

/// Linux keeps it in a directory of numbers, which is the whole of the work here: no call
/// into the kernel, no `unsafe`, and a process that ends halfway through simply stops
/// answering and is left out.
#[cfg(target_os = "linux")]
fn linux() -> Vec<Process> {
    let Ok(dir) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in dir.flatten() {
        let name = entry.file_name();
        let Some(pid) = name.to_str().and_then(|s| s.parse::<u32>().ok()) else {
            continue; // `/proc` holds more than processes
        };
        // One file for all three answers. `status` says them in words and in kibibytes,
        // which saves asking the kernel how big a page is — `statm` counts in pages, and
        // that is four kibibytes on most machines and sixteen or sixty-four on some.
        let Ok(status) = std::fs::read_to_string(entry.path().join("status")) else {
            continue; // it ended while we were reading, which is not an error
        };
        let mut process = Process {
            pid,
            ..Process::default()
        };
        for line in status.lines() {
            if let Some(rest) = line.strip_prefix("Name:") {
                process.name = rest.trim().to_string();
            } else if let Some(rest) = line.strip_prefix("Threads:") {
                process.threads = rest.trim().parse().unwrap_or(0);
            } else if let Some(rest) = line.strip_prefix("VmRSS:") {
                // `VmRSS:\t   12345 kB` — what it has resident, which is the figure worth
                // showing: `VmSize` counts everything it has ever mapped, and for anything
                // with a graphics driver in it that is a number nobody can use.
                let kb = rest
                    .split_whitespace()
                    .next()
                    .and_then(|n| n.parse::<u64>().ok())
                    .unwrap_or(0);
                process.memory = kb * 1024;
            }
        }
        if process.name.is_empty() {
            continue;
        }
        out.push(process);
    }
    out
}

#[cfg(test)]
mod tests {
    // The only test here is Linux's: `/proc` is the one list this crate reads without a
    // call, so it is the one that can be checked against the machine running the tests.
    #[cfg(target_os = "linux")]
    use super::*;

    /// On a machine where the list can be read, we are on it.
    #[test]
    #[cfg(target_os = "linux")]
    fn this_very_process_is_in_the_list() {
        let all = processes();
        assert!(!all.is_empty(), "nothing is running?");
        let me = std::process::id();
        let mine = all.iter().find(|p| p.pid == me).expect("we are running");
        assert!(!mine.name.is_empty());
        // Resident memory of a running test harness is not zero.
        assert!(mine.memory > 0, "{mine:?}");
        assert!(mine.threads > 0, "{mine:?}");
    }

    /// The card's own fields, asked about the one process we are certain of: this one. Every
    /// row here is readable for a process of one's own, which is what the test can promise —
    /// somebody else's answers fewer of them, and the card is written for that.
    #[test]
    #[cfg(target_os = "linux")]
    fn our_own_card_is_filled_in() {
        let me = details(std::process::id()).expect("we are running");
        assert!(me.user.is_some() && me.group.is_some(), "{me:?}");
        assert!(me.priority.is_some(), "{me:?}");
        assert!(me.session.is_some(), "{me:?}");
        assert!(me.cpu_user.is_some() && me.cpu_kernel.is_some(), "{me:?}");
        // A test harness has read something, has opened something, and has an environment.
        assert!(me.handles > 0, "{me:?}");
        assert!(me.faults > 0, "{me:?}");
        assert!(me.private > 0, "{me:?}");
        assert!(!me.env.is_empty(), "{me:?}");
        // Its own executable is the first thing mapped from a file.
        assert!(
            me.modules.iter().any(|m| m.contains("fcmd")),
            "{:?}",
            me.modules
        );
    }
}

/// Everything worth saying about one process: what `F3` on a row of the process panel shows.
///
/// Every field but the pid is optional or zero, and for the same reason the list's are: a
/// process that belongs to somebody else answers some questions and not others, and a field
/// nobody would tell us is left empty rather than filled with a guess.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Details {
    pub pid: u32,
    pub name: String,
    /// What started it, where the system says.
    pub parent: Option<u32>,
    /// Whose it is.
    pub user: Option<String>,
    /// What it is doing, in the system's own words: `S (sleeping)`, `R (running)`.
    pub state: Option<String>,
    pub threads: u32,
    /// Resident memory, in bytes.
    pub memory: u64,
    /// Everything it has mapped, in bytes, which for anything with a graphics driver in it is
    /// a much larger number than the resident one and is worth seeing beside it.
    pub mapped: u64,
    /// 32 or 64, or zero where nothing would say.
    pub bits: u8,
    /// The executable, with its path.
    pub exe: Option<String>,
    /// Which directory it is in.
    pub cwd: Option<String>,
    /// The whole command line, arguments and all.
    pub command: Option<String>,
    pub started: Option<std::time::SystemTime>,
    /// Which group it runs as, where the system has such a thing.
    pub group: Option<String>,
    /// Who it runs as, as the system writes it down: a SID on Windows, and nothing on Unix,
    /// where the name and the numeric id are the whole of it.
    pub sid: Option<String>,
    /// How it is scheduled, in the system's own words: `20 (nice 0)`, `Normal`.
    pub priority: Option<String>,
    /// The login session it belongs to.
    pub session: Option<u32>,
    /// What it has spent in its own code, and what it has spent inside the kernel.
    pub cpu_user: Option<std::time::Duration>,
    pub cpu_kernel: Option<std::time::Duration>,
    /// How many handles it holds — open files on Unix, handles of every kind on Windows.
    pub handles: u32,
    /// How many times a page had to be found for it.
    pub faults: u64,
    /// Memory that is its own rather than shared with anything.
    pub private: u64,
    /// What it has pushed out to swap, or to the paging file.
    pub swap: u64,
    /// Bytes read and written, as the system has counted them.
    pub io_read: u64,
    pub io_write: u64,
    /// The version and the description the executable carries, where it carries them: a
    /// Windows binary has both in its resources, and an ELF has neither.
    pub version: Option<String>,
    pub description: Option<String>,
    /// Its environment, in the order the system keeps it. Empty where it may not be read —
    /// a process of somebody else's never answers this one.
    pub env: Vec<(String, String)>,
    /// Every library it has loaded, by path.
    pub modules: Vec<String>,
}

/// One process, in as much detail as this platform and this user's rights allow.
///
/// `None` means there is no such process, or nothing on this platform can be asked.
pub fn details(pid: u32) -> Option<Details> {
    #[cfg(target_os = "linux")]
    {
        linux_details(pid)
    }
    #[cfg(windows)]
    {
        super::windows::details(pid)
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        let _ = pid;
        None
    }
}

/// Linux again keeps all of it in files, so all of it is reading: `status` for the numbers,
/// `cmdline` for the command, two symbolic links for the executable and the directory, and
/// the directory's own timestamp for when it started — which saves asking the kernel how many
/// clock ticks it counts in a second, the one figure `stat` would need to be read with.
#[cfg(target_os = "linux")]
fn linux_details(pid: u32) -> Option<Details> {
    let dir = std::path::PathBuf::from(format!("/proc/{pid}"));
    let status = std::fs::read_to_string(dir.join("status")).ok()?;
    let mut out = Details {
        pid,
        ..Details::default()
    };
    for line in status.lines() {
        let Some((field, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        let kb = || -> u64 {
            value
                .split_whitespace()
                .next()
                .and_then(|n| n.parse::<u64>().ok())
                .unwrap_or(0)
                * 1024
        };
        match field {
            "Name" => out.name = value.to_string(),
            "State" => out.state = Some(value.to_string()),
            "PPid" => out.parent = value.parse().ok().filter(|p| *p != 0),
            "Threads" => out.threads = value.parse().unwrap_or(0),
            "VmRSS" => out.memory = kb(),
            "VmSize" => out.mapped = kb(),
            // `Uid:\t1000\t1000\t1000\t1000` — the real one is the first.
            "Uid" => {
                out.user = value
                    .split_whitespace()
                    .next()
                    .and_then(|n| n.parse::<u32>().ok())
                    .map(user_name)
            }
            // The same shape as `Uid`, and the same first field: the real group.
            "Gid" => {
                out.group = value
                    .split_whitespace()
                    .next()
                    .and_then(|n| n.parse::<u32>().ok())
                    .map(group_name)
            }
            // What is its own rather than mapped from a file, which is the figure Windows
            // calls private bytes.
            "RssAnon" => out.private = kb(),
            "VmSwap" => out.swap = kb(),
            _ => {}
        }
    }
    // `stat` is one line of numbers whose names are in `proc(5)`. The command is in it too,
    // in brackets, and it may itself contain a bracket — so everything is counted from the
    // last one rather than from the start of the line.
    if let Ok(stat) = std::fs::read_to_string(dir.join("stat")) {
        if let Some(rest) = stat.rsplit_once(')').map(|(_, rest)| rest) {
            // `rest` starts at field 3 (the state), so field N of `proc(5)` is `fields[N-3]`.
            let fields: Vec<&str> = rest.split_whitespace().collect();
            let num = |n: usize| -> Option<u64> { fields.get(n - 3)?.parse::<u64>().ok() };
            out.session = num(6).map(|n| n as u32);
            // Minor and major faults, which is what a page-fault count means here.
            out.faults = num(10).unwrap_or(0) + num(12).unwrap_or(0);
            // The two times are in clock ticks, and how many there are in a second is the
            // one thing about them that is not in the file.
            let ticks = clock_ticks();
            out.cpu_user = num(14).map(|t| ticks_to_time(t, ticks));
            out.cpu_kernel = num(15).map(|t| ticks_to_time(t, ticks));
            // The scheduling priority, and the nice value beside it: one is the other seen
            // from the kernel's end, and a reader of this card wants the one they set.
            if let (Some(priority), Some(nice)) = (
                fields.get(18 - 3).and_then(|n| n.parse::<i64>().ok()),
                fields.get(19 - 3).and_then(|n| n.parse::<i64>().ok()),
            ) {
                out.priority = Some(format!("{priority} (nice {nice})"));
            }
        }
    }
    // What it has read and written. The file is only readable by its owner, so a process of
    // somebody else's leaves these at zero rather than refusing the whole card.
    if let Ok(io) = std::fs::read_to_string(dir.join("io")) {
        for line in io.lines() {
            let Some((field, value)) = line.split_once(':') else {
                continue;
            };
            let bytes = value.trim().parse::<u64>().unwrap_or(0);
            match field {
                // The two that count what actually reached a disk, which is the pair
                // Windows' own IO counters report.
                "read_bytes" => out.io_read = bytes,
                "write_bytes" => out.io_write = bytes,
                _ => {}
            }
        }
    }
    // One entry per open file, which is the nearest thing Linux has to a handle count.
    if let Ok(fds) = std::fs::read_dir(dir.join("fd")) {
        out.handles = fds.flatten().count() as u32;
    }
    // The environment it was given, zero-separated like the command line. Only its owner may
    // read it; anybody else gets an empty list, which is the truth about what we can see.
    if let Ok(raw) = std::fs::read(dir.join("environ")) {
        out.env = raw
            .split(|b| *b == 0)
            .filter(|part| !part.is_empty())
            .filter_map(|part| {
                let text = String::from_utf8_lossy(part);
                text.split_once('=')
                    .map(|(name, value)| (name.to_string(), value.to_string()))
            })
            .collect();
    }
    // Every file-backed mapping, once each and in the order they were mapped: that is the
    // list of libraries it has loaded, with the executable itself at the head of it.
    if let Ok(maps) = std::fs::read_to_string(dir.join("maps")) {
        for line in maps.lines() {
            // `address perms offset dev inode pathname` — a mapping with no file behind it
            // has no last field, and one of those is not a module.
            let Some(path) = line.split_whitespace().nth(5) else {
                continue;
            };
            if !path.starts_with('/') || out.modules.iter().any(|m| m == path) {
                continue;
            }
            out.modules.push(path.to_string());
        }
    }
    // The command line is the arguments with a zero byte between them, and an empty one means
    // a kernel thread, which has no command line at all.
    if let Ok(raw) = std::fs::read(dir.join("cmdline")) {
        let line = raw
            .split(|b| *b == 0)
            .filter(|part| !part.is_empty())
            .map(|part| String::from_utf8_lossy(part).into_owned())
            .collect::<Vec<_>>()
            .join(" ");
        if !line.is_empty() {
            out.command = Some(line);
        }
    }
    let link = |name: &str| -> Option<String> {
        std::fs::read_link(dir.join(name))
            .ok()
            .map(|p| p.to_string_lossy().into_owned())
    };
    out.exe = link("exe");
    out.cwd = link("cwd");
    out.started = std::fs::metadata(&dir).ok().and_then(|m| m.modified().ok());
    // The ELF header of its own executable says which it is: byte four is 1 for a 32-bit
    // object and 2 for a 64-bit one. Reading four bytes is cheaper than a call into the
    // kernel, and a process whose executable we may not read simply does not say.
    out.bits = elf_bits(dir.join("exe")).unwrap_or(0);
    Some(out)
}

/// How many clock ticks the kernel counts in a second, which is what the two times in `stat`
/// are measured in. It is a constant of the running kernel, so it is asked for once.
#[cfg(target_os = "linux")]
fn clock_ticks() -> u64 {
    use std::sync::OnceLock;
    static TICKS: OnceLock<u64> = OnceLock::new();
    *TICKS.get_or_init(|| {
        nix::unistd::sysconf(nix::unistd::SysconfVar::CLK_TCK)
            .ok()
            .flatten()
            .filter(|n| *n > 0)
            .map(|n| n as u64)
            // 100 is what every Linux anybody runs counts in, and a wrong divisor here is a
            // wrong figure on a card rather than a broken program.
            .unwrap_or(100)
    })
}

#[cfg(target_os = "linux")]
fn ticks_to_time(ticks: u64, per_second: u64) -> std::time::Duration {
    let per_second = per_second.max(1);
    std::time::Duration::new(ticks / per_second, 0)
        + std::time::Duration::from_nanos((ticks % per_second) * 1_000_000_000 / per_second)
}

/// The name behind a numeric group id, or the number itself where there is no name for it.
#[cfg(target_os = "linux")]
fn group_name(gid: u32) -> String {
    nix::unistd::Group::from_gid(nix::unistd::Gid::from_raw(gid))
        .ok()
        .flatten()
        .map(|g| g.name)
        .unwrap_or_else(|| gid.to_string())
}

/// The name behind a numeric user id, or the number itself where there is no name for it.
#[cfg(target_os = "linux")]
fn user_name(uid: u32) -> String {
    nix::unistd::User::from_uid(nix::unistd::Uid::from_raw(uid))
        .ok()
        .flatten()
        .map(|u| u.name)
        .unwrap_or_else(|| uid.to_string())
}

/// 32 or 64 from the first five bytes of an ELF file, or `None` for anything else.
#[cfg(target_os = "linux")]
fn elf_bits(path: std::path::PathBuf) -> Option<u8> {
    use std::io::Read;

    let mut head = [0u8; 5];
    let mut file = std::fs::File::open(path).ok()?;
    file.read_exact(&mut head).ok()?;
    if &head[..4] != b"\x7fELF" {
        return None;
    }
    match head[4] {
        1 => Some(32),
        2 => Some(64),
        _ => None,
    }
}
