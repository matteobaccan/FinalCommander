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
