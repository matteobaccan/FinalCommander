//! The programs from elsewhere: every program on the machine that fcmd hands work to rather
//! than doing it itself, found or not, with what it is asked for.
//!
//! The list is not written down twice. The archivers come from the same tables the panels
//! open archives with (`fcmd_arc::tables()`, so an `[[archiver]]` block of one's own is here
//! too), the decompilers from the `[[decompile]]` rules, and the rest are the few programs the
//! code names by hand: `git`, the system's own log, its services and its opener — and
//! `ffmpeg`, which nothing runs yet and which is here because the video frames will.
//!
//! Finding a program is the shell's rule and nothing else ([`fcmd_arc::which`]): what is not
//! on the `PATH` is not installed as far as fcmd can tell, which is what the card says. Its
//! version is asked of the program itself, with a short time limit, off the UI thread.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// What a program is asked for. Each names the place in the program that runs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Use {
    /// `Enter` on an archive no reader of ours knows, through a MultiArc table.
    Archives,
    /// `Shift+F6` in the viewer, through a `[[decompile]]` rule.
    Decompile,
    /// The Git panel (`Ctrl+Alt+G`) and the branch in the prompt.
    Git,
    /// The video frames in the viewer (planned: nothing runs it yet).
    Video,
    /// Opening a link in the desktop's browser.
    Open,
    /// *Commands → System log*.
    SysLog,
    /// The services panel.
    Services,
}

impl Use {
    /// The message that says what it is for.
    pub fn id(self) -> &'static str {
        match self {
            Use::Archives => "tools-for-archives",
            Use::Decompile => "tools-for-decompile",
            Use::Git => "tools-for-git",
            Use::Video => "tools-for-video",
            Use::Open => "tools-for-open",
            Use::SysLog => "tools-for-syslog",
            Use::Services => "tools-for-services",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tool {
    /// The name looked for on the `PATH`.
    pub program: String,
    pub uses: Use,
    /// The extensions (archives) or masks (decompilers) it is asked about.
    pub what: Vec<String>,
    /// The arguments that make it print its version; `None` for the system tools that have
    /// none to ask.
    pub version_args: Option<Vec<String>>,
    /// Where it is, if it is.
    pub found: Option<PathBuf>,
    /// The first line it answered the version question with, once asked.
    pub version: Option<String>,
}

fn tool(program: &str, uses: Use, what: Vec<String>, version: Option<&[&str]>) -> Tool {
    Tool {
        program: program.to_string(),
        uses,
        what,
        version_args: version.map(|args| args.iter().map(|a| a.to_string()).collect()),
        found: None,
        version: None,
    }
}

/// The arguments that make a program say its version. `7z` prints its banner with no
/// arguments at all, `javap` takes one dash, and the rest follow GNU.
fn version_args_for(program: &str) -> &'static [&'static str] {
    let base = Path::new(program)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(program);
    match base {
        "7z" | "7za" | "7zz" => &[],
        "javap" | "java" => &["-version"],
        "ffmpeg" | "ffprobe" => &["-version"],
        _ => &["--version"],
    }
}

/// Every program fcmd may run, in the order the card list shows them: archivers, then git,
/// then the decompilers, then the video, then the system's own tools. Nothing is probed here.
pub fn catalogue(decompile: &[fcmd_config::DecompileRule]) -> Vec<Tool> {
    let mut out: Vec<Tool> = Vec::new();
    for table in fcmd_arc::tables() {
        let exts = table.extensions.clone();
        if let Some(t) = out
            .iter_mut()
            .find(|t| t.uses == Use::Archives && t.program == table.program)
        {
            for e in exts {
                if !t.what.contains(&e) {
                    t.what.push(e);
                }
            }
            continue;
        }
        out.push(tool(
            &table.program,
            Use::Archives,
            exts,
            Some(version_args_for(&table.program)),
        ));
    }
    out.push(tool("git", Use::Git, Vec::new(), Some(&["--version"])));
    for rule in decompile {
        let program = rule.program().to_string();
        if program.is_empty() {
            continue;
        }
        if let Some(t) = out
            .iter_mut()
            .find(|t| t.uses == Use::Decompile && t.program == program)
        {
            if !t.what.contains(&rule.mask) {
                t.what.push(rule.mask.clone());
            }
            continue;
        }
        out.push(tool(
            &program,
            Use::Decompile,
            vec![rule.mask.clone()],
            Some(version_args_for(&program)),
        ));
    }
    out.push(tool("ffmpeg", Use::Video, Vec::new(), Some(&["-version"])));
    if cfg!(windows) {
        out.push(tool("wevtutil", Use::SysLog, Vec::new(), None));
    } else if cfg!(target_os = "macos") {
        out.push(tool("open", Use::Open, Vec::new(), None));
        out.push(tool("log", Use::SysLog, Vec::new(), None));
        out.push(tool("launchctl", Use::Services, Vec::new(), None));
    } else {
        out.push(tool(
            "xdg-open",
            Use::Open,
            Vec::new(),
            Some(&["--version"]),
        ));
        out.push(tool(
            "journalctl",
            Use::SysLog,
            Vec::new(),
            Some(&["--version"]),
        ));
        out.push(tool(
            "systemctl",
            Use::Services,
            Vec::new(),
            Some(&["--version"]),
        ));
    }
    out
}

/// Look every program up on the `PATH`: instant, so done where the panel is built.
pub fn locate(tools: &mut [Tool]) {
    for t in tools {
        t.found = fcmd_arc::which(&t.program);
    }
}

/// How long a program is given to say its version. `javap` and `ilspycmd` start a runtime
/// first; anything slower than this is reported as found, without a version.
pub const VERSION_WAIT: Duration = Duration::from_secs(3);

/// Ask a program its version: the first line it prints that is not blank, on either stream.
pub fn version_of(path: &Path, args: &[String], wait: Duration) -> Option<String> {
    use std::process::{Command, Stdio};
    let mut child = Command::new(path)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    let mut out = child.stdout.take()?;
    let mut err = child.stderr.take()?;
    // Read both streams on their own threads, so a program that fills one while we wait on
    // the other never blocks either of us.
    let read_out = std::thread::spawn(move || {
        let mut s = Vec::new();
        let _ = out.read_to_end(&mut s);
        s
    });
    let read_err = std::thread::spawn(move || {
        let mut s = Vec::new();
        let _ = err.read_to_end(&mut s);
        s
    });
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if started.elapsed() < wait => {
                std::thread::sleep(Duration::from_millis(20));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
    let mut text = read_out.join().unwrap_or_default();
    text.extend(read_err.join().unwrap_or_default());
    first_line(&String::from_utf8_lossy(&text))
}

/// The line worth showing: the first that is not blank, trimmed, and short enough for a
/// column.
pub fn first_line(text: &str) -> Option<String> {
    let line = text.lines().map(str::trim).find(|l| !l.is_empty())?;
    const MOST: usize = 72;
    if line.chars().count() > MOST {
        Some(line.chars().take(MOST - 1).collect::<String>() + "…")
    } else {
        Some(line.to_string())
    }
}

/// The package managers an install line is written for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Manager {
    Apt,
    Dnf,
    Pacman,
    Apk,
    Brew,
    Winget,
}

/// The package manager this machine has, by the same `PATH` rule.
pub fn manager() -> Option<Manager> {
    let order: &[(&str, Manager)] = if cfg!(windows) {
        &[("winget", Manager::Winget)]
    } else if cfg!(target_os = "macos") {
        &[("brew", Manager::Brew)]
    } else {
        &[
            ("apt-get", Manager::Apt),
            ("dnf", Manager::Dnf),
            ("pacman", Manager::Pacman),
            ("apk", Manager::Apk),
        ]
    };
    order
        .iter()
        .find(|(program, _)| fcmd_arc::which(program).is_some())
        .map(|(_, m)| *m)
}

/// The command that installs `program` with this manager, when we know one that is right.
///
/// Only names we are sure of: a wrong package name is worse than none, because the card then
/// says what to look for instead. `dnf` is the exception that needs no table, because it
/// installs by the file a package provides.
pub fn install_command(program: &str, manager: Option<Manager>) -> Option<String> {
    // The one that is a tool of a tool, the same on every system.
    if program == "ilspycmd" {
        return Some("dotnet tool install --global ilspycmd".to_string());
    }
    // Part of the system: nothing to install.
    if matches!(
        program,
        "journalctl" | "systemctl" | "open" | "log" | "launchctl" | "wevtutil"
    ) {
        return None;
    }
    let manager = manager?;
    let package = match manager {
        Manager::Dnf => return Some(format!("sudo dnf install /usr/bin/{program}")),
        Manager::Apt => match program {
            "7z" => "p7zip-full",
            "git" => "git",
            "ffmpeg" => "ffmpeg",
            "javap" => "default-jdk-headless",
            "python3" => "python3",
            "wasm2wat" => "wabt",
            "objdump" => "binutils",
            "xdg-open" => "xdg-utils",
            _ => return None,
        },
        Manager::Pacman => match program {
            "git" => "git",
            "ffmpeg" => "ffmpeg",
            "javap" => "jdk-openjdk",
            "python3" => "python",
            "wasm2wat" => "wabt",
            "objdump" => "binutils",
            "xdg-open" => "xdg-utils",
            _ => return None,
        },
        Manager::Apk => match program {
            "git" => "git",
            "ffmpeg" => "ffmpeg",
            "python3" => "python3",
            "objdump" => "binutils",
            "xdg-open" => "xdg-utils",
            _ => return None,
        },
        Manager::Brew => match program {
            "git" => "git",
            "ffmpeg" => "ffmpeg",
            "python3" => "python",
            "wasm2wat" => "wabt",
            _ => return None,
        },
        Manager::Winget => match program {
            "7z" => "7zip.7zip",
            "git" => "Git.Git",
            "ffmpeg" => "Gyan.FFmpeg",
            "javap" => "Microsoft.OpenJDK.21",
            "python3" => "Python.Python.3.12",
            _ => return None,
        },
    };
    Some(match manager {
        Manager::Apt => format!("sudo apt install {package}"),
        Manager::Pacman => format!("sudo pacman -S {package}"),
        Manager::Apk => format!("sudo apk add {package}"),
        Manager::Brew => format!("brew install {package}"),
        Manager::Winget => format!("winget install --exact --id {package}"),
        Manager::Dnf => unreachable!("answered above"),
    })
}

/// Where a program comes from, for when no package manager here has it or the install
/// failed: the product's own download page, never a mirror.
pub fn homepage(program: &str) -> Option<&'static str> {
    Some(match program {
        "7z" | "7za" | "7zz" => "https://www.7-zip.org/download.html",
        "git" => "https://git-scm.com/downloads",
        "ffmpeg" | "ffprobe" => "https://ffmpeg.org/download.html",
        "javap" => "https://adoptium.net/",
        "ilspycmd" => "https://github.com/icsharpcode/ILSpy",
        "python3" => "https://www.python.org/downloads/",
        "wasm2wat" => "https://github.com/WebAssembly/wabt/releases",
        "objdump" => "https://www.gnu.org/software/binutils/",
        "retdec-decompiler" => "https://github.com/avast/retdec/releases",
        "xdg-open" => "https://www.freedesktop.org/wiki/Software/xdg-utils/",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_catalogue_names_what_the_code_runs() {
        let rules = fcmd_config::default_decompile_rules();
        let list = catalogue(&rules);
        let has = |p: &str, u: Use| list.iter().any(|t| t.program == p && t.uses == u);
        assert!(has("7z", Use::Archives));
        assert!(has("git", Use::Git));
        assert!(has("ffmpeg", Use::Video));
        assert!(has("javap", Use::Decompile));
        // One row per program: objdump answers three masks.
        let objdump: Vec<_> = list.iter().filter(|t| t.program == "objdump").collect();
        assert_eq!(objdump.len(), 1);
        assert!(objdump[0].what.len() >= 3, "{:?}", objdump[0].what);
        let seven = list.iter().find(|t| t.program == "7z").unwrap();
        assert!(seven.what.iter().any(|e| e == "rar"));
    }

    #[test]
    fn a_version_is_the_first_line_that_says_something() {
        assert_eq!(
            first_line("\n  \n7-Zip 23.01 (x64)\nusage").as_deref(),
            Some("7-Zip 23.01 (x64)")
        );
        assert_eq!(first_line("   \n"), None);
        let long = "x".repeat(200);
        assert_eq!(first_line(&long).unwrap().chars().count(), 72);
    }

    #[test]
    fn install_lines_only_where_the_name_is_sure() {
        assert_eq!(
            install_command("7z", Some(Manager::Apt)).as_deref(),
            Some("sudo apt install p7zip-full")
        );
        assert_eq!(
            install_command("wasm2wat", Some(Manager::Dnf)).as_deref(),
            Some("sudo dnf install /usr/bin/wasm2wat")
        );
        assert_eq!(
            install_command("retdec-decompiler", Some(Manager::Apt)),
            None
        );
        assert_eq!(install_command("journalctl", Some(Manager::Apt)), None);
        assert_eq!(install_command("git", None), None);
        assert_eq!(
            install_command("7z", Some(Manager::Winget)).as_deref(),
            Some("winget install --exact --id 7zip.7zip")
        );
        assert!(homepage("7z").unwrap().starts_with("https://"));
        assert_eq!(homepage("journalctl"), None);
        assert!(install_command("ilspycmd", None)
            .unwrap()
            .starts_with("dotnet tool install"));
    }

    /// A real program on this machine answers; one that is not there is not found.
    #[cfg(unix)]
    #[test]
    fn a_program_on_the_path_is_found_and_asked() {
        let mut list = vec![
            tool(
                "sh",
                Use::Git,
                Vec::new(),
                Some(&["-c", "echo '  '; echo shell 1.0"]),
            ),
            tool(
                "surely-not-a-program-here",
                Use::Video,
                Vec::new(),
                Some(&[]),
            ),
        ];
        locate(&mut list);
        let sh = list[0].found.clone().expect("sh is on every Unix PATH");
        assert_eq!(
            version_of(&sh, list[0].version_args.as_ref().unwrap(), VERSION_WAIT).as_deref(),
            Some("shell 1.0")
        );
        assert!(list[1].found.is_none());
        // And one that does not answer in time is given up on, not waited for.
        let started = Instant::now();
        assert_eq!(
            version_of(
                &sh,
                &["-c".into(), "sleep 5".into()],
                Duration::from_millis(200)
            ),
            None
        );
        assert!(started.elapsed() < Duration::from_secs(3));
    }
}
