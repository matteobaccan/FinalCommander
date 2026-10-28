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
            _ => {}
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
