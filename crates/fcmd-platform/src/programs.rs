//! What is installed on this machine (SPEC 02 §7d).
//!
//! FAR has a plugin for this — `UnInstall` — and it earns its place for the same reason the
//! process panel does: the list is somewhere the system keeps it, nobody remembers where, and
//! what one wants is to sort it by size and see what is taking the disk. Windows keeps it in
//! the registry, under the key the Control Panel's own list reads; a Linux machine keeps it in
//! its package manager's database.
//!
//! Nothing here removes anything. The command that would is *reported* — it is the one piece
//! of information the list has that cannot be found any other way — and goes on the command
//! line for the person at the keyboard to read before pressing `Enter`.

/// One installed program, as the panel shows it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Program {
    pub name: String,
    pub version: Option<String>,
    /// Who made it: the publisher on Windows, the maintainer of a package elsewhere.
    pub publisher: Option<String>,
    /// What it takes on disk, in bytes, as far as the system will say. Zero where it will not.
    pub size: u64,
    /// When it was installed, as a date, where the system keeps one.
    pub installed: Option<String>,
    /// Where it lives, where the system keeps that.
    pub location: Option<String>,
    /// The command that would remove it — Windows keeps the real one; elsewhere it is the
    /// package manager's own remove command, which is the same thing said in its words.
    pub uninstall: Option<String>,
}

/// Everything installed, as far as this user is allowed to see.
pub fn programs() -> Vec<Program> {
    #[cfg(windows)]
    {
        super::windows::programs()
    }
    #[cfg(target_os = "linux")]
    {
        linux()
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        // macOS keeps applications as directories under `/Applications`, which is a listing
        // rather than a database and is better served by the panel one already has.
        Vec::new()
    }
}

/// Linux has no one answer, so the two databases that are plain files are read: dpkg's status
/// file and pacman's directory of descriptions. An rpm database is a Berkeley DB or sqlite
/// behind `librpm` and waits for a tested way to read it (`docs/BACKLOG.md`).
#[cfg(target_os = "linux")]
fn linux() -> Vec<Program> {
    let mut out = dpkg();
    if out.is_empty() {
        out = pacman();
    }
    out
}

/// `/var/lib/dpkg/status`: one paragraph per package, `Field: value`, continuation lines
/// indented. Only what is really installed counts — the file keeps packages that were removed
/// but left their configuration behind.
#[cfg(target_os = "linux")]
fn dpkg() -> Vec<Program> {
    let Ok(text) = std::fs::read_to_string("/var/lib/dpkg/status") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for block in text.split("\n\n") {
        let mut program = Program::default();
        let mut installed = false;
        for line in block.lines() {
            let Some((field, value)) = line.split_once(':') else {
                continue;
            };
            let value = value.trim();
            match field {
                "Package" => program.name = value.to_string(),
                "Version" => program.version = Some(value.to_string()),
                "Maintainer" => program.publisher = Some(value.to_string()),
                // dpkg counts in kibibytes.
                "Installed-Size" => {
                    program.size = value.parse::<u64>().unwrap_or(0) * 1024;
                }
                "Status" => installed = value.contains("install ok installed"),
                _ => {}
            }
        }
        if program.name.is_empty() || !installed {
            continue;
        }
        program.uninstall = Some(format!("apt-get remove {}", program.name));
        out.push(program);
    }
    out
}

/// `/var/lib/pacman/local/<name>-<version>/desc`: `%FIELD%` on one line, the value on the
/// next.
#[cfg(target_os = "linux")]
fn pacman() -> Vec<Program> {
    let Ok(dir) = std::fs::read_dir("/var/lib/pacman/local") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in dir.flatten() {
        let Ok(text) = std::fs::read_to_string(entry.path().join("desc")) else {
            continue;
        };
        let mut program = Program::default();
        let mut lines = text.lines();
        while let Some(line) = lines.next() {
            let Some(value) = lines.clone().next() else {
                break;
            };
            match line {
                "%NAME%" => program.name = value.to_string(),
                "%VERSION%" => program.version = Some(value.to_string()),
                "%PACKAGER%" => program.publisher = Some(value.to_string()),
                "%SIZE%" => program.size = value.parse().unwrap_or(0),
                _ => continue,
            }
            lines.next();
        }
        if program.name.is_empty() {
            continue;
        }
        program.uninstall = Some(format!("pacman -R {}", program.name));
        out.push(program);
    }
    out
}

#[cfg(test)]
mod tests {
    /// Whatever this machine has, every row says what it is and the size is a size.
    #[test]
    fn what_is_installed_reads_as_a_list_of_programs() {
        for program in super::programs() {
            assert!(!program.name.is_empty());
            // A package with no uninstall command is one we could not say how to remove,
            // which is allowed; a command that is there has to name the program.
            if let Some(command) = &program.uninstall {
                assert!(command.contains(&program.name), "{program:?}");
            }
        }
    }
}
