//! What starts when this user logs in, and switching it off (SPEC 02 §14).
//!
//! Every system keeps the list in more than one place, and none of them in one a person
//! remembers: Windows in four registry keys, two folders and the task scheduler, a Linux
//! desktop in two `autostart` folders and the user's systemd units, macOS in three
//! `Launch*` folders. This reads all of them into one list with the same four facts for
//! each row — what it is called, what it runs, where it comes from, and whether it is on.
//!
//! **Nothing is deleted to switch something off.** Each system has its own way of saying
//! "not at login" that leaves the entry where it is, and that is what is used, so that the
//! system's own tools show the same state this panel does: Task Manager's `StartupApproved`
//! values on Windows, a `Hidden=true` copy in the user's own folder on a Linux desktop (what
//! GNOME and KDE write), `systemctl --user disable`, and `launchctl disable`. Removing an
//! entry is a separate key that asks first.
//!
//! **Nothing here runs as anybody else**, as with the services: what belongs to the machine
//! is changed only where this user may change it, and a refusal is reported as one.

use std::path::{Path, PathBuf};

pub use crate::service::Error as StartupError;

/// Where one entry lives, which is also how it is switched on and off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartupWhere {
    /// A value under a `Run` or `RunOnce` key (Windows). `machine` is `HKLM`, `wow` the
    /// 32-bit view of it, `value` the value's name.
    Run {
        machine: bool,
        once: bool,
        wow: bool,
        value: String,
    },
    /// A file in a *Startup* folder (Windows): the user's, or the one every user shares.
    Folder { machine: bool, file: PathBuf },
    /// A scheduled task that runs at logon (Windows), by its path in the scheduler.
    Task { name: String, file: PathBuf },
    /// A `.desktop` file in an `autostart` folder (Linux). `user` is the file in this user's
    /// folder, which may not exist yet; `system` the one in the machine's, if there is one.
    Desktop {
        user: PathBuf,
        system: Option<PathBuf>,
    },
    /// A `systemd --user` unit (Linux).
    Systemd { unit: String, file: Option<PathBuf> },
    /// A launchd job (macOS): an agent of this user, an agent of every user, or a daemon.
    Launchd {
        label: String,
        file: PathBuf,
        user: bool,
    },
}

/// One thing that starts at login, as the panel shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Startup {
    /// What it is called: a value's name, a file's, a unit's, a job's label.
    pub name: String,
    /// What it runs, as far as the place it lives says.
    pub command: String,
    /// Where it comes from, in the words of the system: `HKCU\…\Run`, `~/.config/autostart`.
    pub source: String,
    /// Whether it starts at the next login.
    pub enabled: bool,
    /// It belongs to the machine rather than to this user, so changing it may want root or
    /// an administrator.
    pub machine: bool,
    /// Who made it, where the place it lives says.
    pub publisher: Option<String>,
    pub place: StartupWhere,
}

impl Startup {
    /// The file that defines it, for `F4`. A registry value has none.
    pub fn file(&self) -> Option<&Path> {
        match &self.place {
            StartupWhere::Run { .. } => None,
            StartupWhere::Folder { file, .. } | StartupWhere::Task { file, .. } => Some(file),
            StartupWhere::Desktop { user, system } => {
                if user.exists() {
                    Some(user)
                } else {
                    system.as_deref()
                }
            }
            StartupWhere::Systemd { file, .. } => file.as_deref(),
            StartupWhere::Launchd { file, .. } => Some(file),
        }
    }
}

/// Everything that starts at login, as far as this user may see.
pub fn startup() -> Vec<Startup> {
    #[cfg(windows)]
    {
        crate::windows::startup()
    }
    #[cfg(target_os = "linux")]
    {
        let mut out = desktop_entries(&user_autostart(), &system_autostart());
        out.extend(systemd_user());
        out
    }
    #[cfg(target_os = "macos")]
    {
        launchd_jobs()
    }
    #[cfg(not(any(target_os = "linux", windows, target_os = "macos")))]
    {
        Vec::new()
    }
}

/// Switch one on or off for the next login, the way the system's own tools do.
pub fn set_startup(item: &Startup, on: bool) -> Result<(), StartupError> {
    match &item.place {
        StartupWhere::Desktop { user, system } => set_desktop(user, system.as_deref(), on),
        #[cfg(target_os = "linux")]
        StartupWhere::Systemd { unit, .. } => {
            checked_name(unit)?;
            run(
                "systemctl",
                &["--user", if on { "enable" } else { "disable" }, unit],
            )
        }
        #[cfg(target_os = "macos")]
        StartupWhere::Launchd { label, user, .. } => {
            checked_name(label)?;
            let target = if *user {
                format!("gui/{}/{label}", nix::unistd::getuid())
            } else {
                format!("system/{label}")
            };
            run(
                "launchctl",
                &[if on { "enable" } else { "disable" }, &target],
            )
        }
        #[cfg(windows)]
        _ => crate::windows::set_startup(item, on),
        #[cfg(not(windows))]
        _ => Err(StartupError::Unsupported),
    }
}

/// Take one away for good: the registry value, the file, the task. A machine's entry on a
/// Linux desktop is not removed — it is the machine's file, and switching it off is the
/// user's own copy of it — and a systemd unit is disabled rather than deleted.
pub fn remove_startup(item: &Startup) -> Result<(), StartupError> {
    match &item.place {
        StartupWhere::Desktop { user, system } => {
            if system.is_some() {
                return Err(StartupError::NeedsRoot);
            }
            remove_file(user)
        }
        StartupWhere::Systemd { .. } => Err(StartupError::Unsupported),
        StartupWhere::Launchd { file, .. } => remove_file(file),
        StartupWhere::Folder { file, .. } => remove_file(file),
        #[cfg(windows)]
        _ => crate::windows::remove_startup(item),
        #[cfg(not(windows))]
        _ => Err(StartupError::Unsupported),
    }
}

fn remove_file(path: &Path) -> Result<(), StartupError> {
    std::fs::remove_file(path).map_err(io_error)
}

fn io_error(e: std::io::Error) -> StartupError {
    if e.kind() == std::io::ErrorKind::PermissionDenied {
        StartupError::NeedsRoot
    } else {
        StartupError::Failed(e.to_string())
    }
}

/// A name handed to a command line is checked first, as a service's is.
#[cfg_attr(windows, allow(dead_code))]
fn checked_name(name: &str) -> Result<(), StartupError> {
    let ok = !name.is_empty()
        && !name.starts_with('-')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '@' | ':'));
    if ok {
        Ok(())
    } else {
        Err(StartupError::Failed(format!("`{name}` is not a name")))
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn run(program: &str, args: &[&str]) -> Result<(), StartupError> {
    let out = match std::process::Command::new(program).args(args).output() {
        Ok(out) => out,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(StartupError::Unsupported)
        }
        Err(e) => return Err(StartupError::Failed(e.to_string())),
    };
    if out.status.success() {
        return Ok(());
    }
    let said = String::from_utf8_lossy(&out.stderr);
    let lower = said.to_ascii_lowercase();
    if lower.contains("permission")
        || lower.contains("not permitted")
        || lower.contains("access denied")
    {
        return Err(StartupError::NeedsRoot);
    }
    let line = said.lines().next().unwrap_or("").trim();
    Err(StartupError::Failed(if line.is_empty() {
        format!("{program} refused")
    } else {
        line.to_string()
    }))
}

// ── Linux: the XDG autostart folders ────────────────────────────────────────────────────

/// `$XDG_CONFIG_HOME/autostart`, or `~/.config/autostart`.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn user_autostart() -> PathBuf {
    match std::env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        Some(dir) => PathBuf::from(dir).join("autostart"),
        None => PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
            .join(".config")
            .join("autostart"),
    }
}

/// `autostart` under each of `$XDG_CONFIG_DIRS`, or `/etc/xdg/autostart`.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn system_autostart() -> Vec<PathBuf> {
    let dirs = std::env::var("XDG_CONFIG_DIRS")
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "/etc/xdg".into());
    dirs.split(':')
        .filter(|d| !d.is_empty())
        .map(|d| PathBuf::from(d).join("autostart"))
        .collect()
}

/// What a `.desktop` file says about itself, as far as autostart cares.
#[derive(Debug, Default, PartialEq, Eq)]
struct DesktopFile {
    name: String,
    exec: String,
    hidden: bool,
    /// GNOME's own switch, which it writes instead of `Hidden` in some versions.
    gnome_off: bool,
}

/// The `[Desktop Entry]` group of a `.desktop` file. Other groups — actions — are skipped, and
/// a localised `Name[it]` is not the name.
fn parse_desktop(text: &str) -> DesktopFile {
    let mut out = DesktopFile::default();
    let mut in_entry = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_entry || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        match key {
            "Name" => out.name = value.to_string(),
            "Exec" => out.exec = value.to_string(),
            "Hidden" => out.hidden = value.eq_ignore_ascii_case("true"),
            "X-GNOME-Autostart-enabled" => out.gnome_off = value.eq_ignore_ascii_case("false"),
            _ => {}
        }
    }
    out
}

/// The same file with `Hidden` set to `hidden` — the line rewritten where there is one,
/// added at the end of `[Desktop Entry]` where there is not — and GNOME's own switch
/// following it, so the two never say different things.
fn with_hidden(text: &str, hidden: bool) -> String {
    let value = if hidden { "true" } else { "false" };
    let mut out: Vec<String> = Vec::new();
    let mut in_entry = false;
    let mut wrote = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            if in_entry && !wrote {
                out.push(format!("Hidden={value}"));
                wrote = true;
            }
            in_entry = trimmed == "[Desktop Entry]";
            out.push(line.to_string());
            continue;
        }
        if in_entry {
            if let Some((key, _)) = trimmed.split_once('=') {
                match key.trim() {
                    "Hidden" => {
                        if !wrote {
                            out.push(format!("Hidden={value}"));
                            wrote = true;
                        }
                        continue;
                    }
                    "X-GNOME-Autostart-enabled" => {
                        out.push(format!("X-GNOME-Autostart-enabled={}", !hidden));
                        continue;
                    }
                    _ => {}
                }
            }
        }
        out.push(line.to_string());
    }
    if !wrote {
        if !in_entry {
            out.push("[Desktop Entry]".to_string());
        }
        out.push(format!("Hidden={value}"));
    }
    let mut text = out.join("\n");
    text.push('\n');
    text
}

/// Every autostart entry: the user's files, and the machine's that the user has not
/// overridden with a file of the same name — which is the rule the desktops follow.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn desktop_entries(user_dir: &Path, system_dirs: &[PathBuf]) -> Vec<Startup> {
    let mut by_name: std::collections::BTreeMap<String, (Option<PathBuf>, Option<PathBuf>)> =
        std::collections::BTreeMap::new();
    let mut scan = |dir: &Path, user: bool| {
        let Ok(read) = std::fs::read_dir(dir) else {
            return;
        };
        for item in read.flatten() {
            let path = item.path();
            if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
                continue;
            }
            let Some(file) = path.file_name().map(|f| f.to_string_lossy().into_owned()) else {
                continue;
            };
            let slot = by_name.entry(file).or_default();
            if user {
                slot.0 = Some(path);
            } else if slot.1.is_none() {
                // The first of `$XDG_CONFIG_DIRS` that has it wins, as in the spec.
                slot.1 = Some(path);
            }
        }
    };
    scan(user_dir, true);
    for dir in system_dirs {
        scan(dir, false);
    }
    let mut out = Vec::new();
    for (file, (user, system)) in by_name {
        let Some(read_from) = user.as_ref().or(system.as_ref()) else {
            continue;
        };
        let Ok(text) = std::fs::read_to_string(read_from) else {
            continue;
        };
        let desktop = parse_desktop(&text);
        let name = if desktop.name.is_empty() {
            file.trim_end_matches(".desktop").to_string()
        } else {
            desktop.name.clone()
        };
        let source = read_from
            .parent()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        out.push(Startup {
            name,
            command: desktop.exec.clone(),
            source,
            enabled: !desktop.hidden && !desktop.gnome_off,
            machine: user.is_none(),
            publisher: None,
            place: StartupWhere::Desktop {
                user: user.unwrap_or_else(|| user_dir.join(&file)),
                system,
            },
        });
    }
    out
}

/// On or off for a `.desktop` entry: the user's file is rewritten, or — for one only the
/// machine has — a copy of the machine's is written into the user's folder with `Hidden` set,
/// which is what the desktops' own settings do.
fn set_desktop(user: &Path, system: Option<&Path>, on: bool) -> Result<(), StartupError> {
    let from = if user.exists() {
        user
    } else {
        match system {
            Some(system) => system,
            None => return Err(StartupError::Failed("the file is gone".into())),
        }
    };
    let text = std::fs::read_to_string(from).map_err(io_error)?;
    if let Some(dir) = user.parent() {
        std::fs::create_dir_all(dir).map_err(io_error)?;
    }
    std::fs::write(user, with_hidden(&text, !on)).map_err(io_error)
}

// ── Linux: systemd --user ───────────────────────────────────────────────────────────────

/// The user's own units that are enabled or disabled — the ones `enable` and `disable` mean
/// anything for — with what each runs, read in two calls for the lot.
#[cfg(target_os = "linux")]
fn systemd_user() -> Vec<Startup> {
    use std::process::Command;

    let Ok(out) = Command::new("systemctl")
        .args([
            "--user",
            "list-unit-files",
            "--type=service",
            "--no-legend",
            "--plain",
            "--no-pager",
        ])
        .output()
    else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    let units = user_unit_files(&String::from_utf8_lossy(&out.stdout));
    if units.is_empty() {
        return Vec::new();
    }
    let mut args = vec![
        "--user".to_string(),
        "show".to_string(),
        "--property=Id,Description,ExecStart,FragmentPath".to_string(),
    ];
    args.extend(units.iter().map(|(unit, _)| unit.clone()));
    let shown = Command::new("systemctl")
        .args(&args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| unit_properties(&String::from_utf8_lossy(&o.stdout)))
        .unwrap_or_default();
    units
        .into_iter()
        .map(|(unit, enabled)| {
            let props = shown.get(&unit);
            let get = |key: &str| props.and_then(|p| p.get(key)).cloned().unwrap_or_default();
            let file = Some(get("FragmentPath"))
                .filter(|f| !f.is_empty())
                .map(PathBuf::from);
            Startup {
                name: unit.clone(),
                command: exec_line(&get("ExecStart")),
                source: "systemd --user".into(),
                enabled,
                // A unit the machine ships for every user is still this user's to enable.
                machine: false,
                publisher: Some(get("Description")).filter(|d| !d.is_empty()),
                place: StartupWhere::Systemd { unit, file },
            }
        })
        .collect()
}

/// `UNIT STATE PRESET` rows: the enabled and the disabled ones, which are the two states
/// a user switches between.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn user_unit_files(text: &str) -> Vec<(String, bool)> {
    text.lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let (unit, state) = (parts.next()?, parts.next()?);
            if unit.ends_with("@.service") {
                return None; // a template, which is enabled through its instances
            }
            match state {
                "enabled" => Some((unit.to_string(), true)),
                "disabled" => Some((unit.to_string(), false)),
                _ => None,
            }
        })
        .collect()
}

/// `systemctl show` for several units: blocks of `Key=value` separated by blank lines, each
/// with its `Id`.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn unit_properties(
    text: &str,
) -> std::collections::HashMap<String, std::collections::HashMap<String, String>> {
    let mut out = std::collections::HashMap::new();
    for block in text.split("\n\n") {
        let props: std::collections::HashMap<String, String> = block
            .lines()
            .filter_map(|l| l.split_once('='))
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        if let Some(id) = props.get("Id").cloned() {
            out.insert(id, props);
        }
    }
    out
}

/// systemd's `ExecStart` property — `{ path=/usr/bin/x ; argv[]=/usr/bin/x --flag ; … }` —
/// as the command line in it.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn exec_line(raw: &str) -> String {
    match raw.split_once("argv[]=") {
        Some((_, rest)) => rest.split(" ;").next().unwrap_or("").trim().to_string(),
        None => raw.trim().to_string(),
    }
}

// ── macOS: launchd ──────────────────────────────────────────────────────────────────────

/// The three folders launchd reads at login and at boot, and the jobs in each.
#[cfg(target_os = "macos")]
fn launchd_jobs() -> Vec<Startup> {
    let uid = nix::unistd::getuid();
    let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
    let disabled_in = |domain: &str| -> std::collections::HashSet<String> {
        std::process::Command::new("launchctl")
            .args(["print-disabled", domain])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| crate::service::launchd_disabled(&String::from_utf8_lossy(&o.stdout)))
            .unwrap_or_default()
    };
    let user_off = disabled_in(&format!("gui/{uid}"));
    let system_off = disabled_in("system");
    let mut out = Vec::new();
    for (dir, user) in [
        (home.join("Library/LaunchAgents"), true),
        (PathBuf::from("/Library/LaunchAgents"), true),
        (PathBuf::from("/Library/LaunchDaemons"), false),
    ] {
        let machine = !dir.starts_with(&home);
        let Ok(read) = std::fs::read_dir(&dir) else {
            continue;
        };
        for item in read.flatten() {
            let path = item.path();
            if path.extension().and_then(|e| e.to_str()) != Some("plist") {
                continue;
            }
            let bytes = std::fs::read(&path).unwrap_or_default();
            let job = parse_plist(&String::from_utf8_lossy(&bytes));
            let label = job.label.clone().unwrap_or_else(|| {
                path.file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default()
            });
            let off = if user { &user_off } else { &system_off };
            out.push(Startup {
                name: label.clone(),
                command: job.command,
                source: dir.display().to_string(),
                enabled: !job.disabled && !off.contains(&label),
                machine,
                publisher: None,
                place: StartupWhere::Launchd {
                    label,
                    file: path,
                    user,
                },
            });
        }
    }
    out
}

/// What a launchd job's property list says, as far as this panel cares.
#[derive(Debug, Default, PartialEq, Eq)]
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
struct Plist {
    label: Option<String>,
    command: String,
    disabled: bool,
}

/// The XML property list a launchd job is written in, read for the four keys that matter:
/// `Label`, `Program`, `ProgramArguments` and `Disabled`. A binary list — `bplist00` — says
/// nothing here, and the job is shown by its file's name.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn parse_plist(text: &str) -> Plist {
    let mut out = Plist::default();
    if !text.contains("<plist") {
        return out;
    }
    let value_after = |key: &str| -> Option<&str> {
        let at = text.find(&format!("<key>{key}</key>"))? + key.len() + 11;
        Some(text[at..].trim_start())
    };
    let string_in = |rest: &str| -> Option<String> {
        let rest = rest.strip_prefix("<string>")?;
        let end = rest.find("</string>")?;
        Some(xml_text(&rest[..end]))
    };
    out.label = value_after("Label").and_then(string_in);
    if let Some(rest) = value_after("ProgramArguments") {
        if let Some(body) = rest.strip_prefix("<array>") {
            let body = &body[..body.find("</array>").unwrap_or(body.len())];
            let mut args = Vec::new();
            let mut at = body;
            while let Some(start) = at.find("<string>") {
                let after = &at[start + 8..];
                let Some(end) = after.find("</string>") else {
                    break;
                };
                args.push(xml_text(&after[..end]));
                at = &after[end + 9..];
            }
            out.command = args.join(" ");
        }
    }
    if out.command.is_empty() {
        if let Some(program) = value_after("Program").and_then(string_in) {
            out.command = program;
        }
    }
    out.disabled = value_after("Disabled").is_some_and(|rest| rest.starts_with("<true"));
    out
}

/// The five entities XML has, which is all a property list uses.
fn xml_text(raw: &str) -> String {
    raw.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

// ── Windows: what Task Manager's list reads ─────────────────────────────────────────────

/// Task Manager's verdict on one entry, from its `StartupApproved` value: twelve bytes whose
/// first is even for on and odd for off. No value at all is on, which is every entry Task
/// Manager has never been asked about.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn approved(value: Option<&[u8]>) -> bool {
    match value.and_then(|v| v.first()) {
        Some(first) => first % 2 == 0,
        None => true,
    }
}

/// The twelve bytes Task Manager writes: `02` for on, `03` for off, then the moment it was
/// switched off as a `FILETIME` (zero for on).
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn approval_bytes(on: bool, filetime: u64) -> [u8; 12] {
    let mut out = [0u8; 12];
    out[0] = if on { 2 } else { 3 };
    if !on {
        out[4..].copy_from_slice(&filetime.to_le_bytes());
    }
    out
}

/// What a scheduled task's XML says, for the ones that run at logon.
#[derive(Debug, Default, PartialEq, Eq)]
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) struct TaskXml {
    pub at_logon: bool,
    pub enabled: bool,
    pub command: String,
    pub author: Option<String>,
}

/// A task file, which the scheduler writes as UTF-16 with a byte order mark (and an export
/// may be UTF-8), read for its logon trigger, its `Settings/Enabled`, its command and its
/// author.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn parse_task(bytes: &[u8]) -> TaskXml {
    let text = if bytes.starts_with(&[0xFF, 0xFE]) {
        let wide: Vec<u16> = bytes[2..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_le_bytes(*c))
            .collect();
        String::from_utf16_lossy(&wide)
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    };
    let between = |from: &str, open: &str, close: &str| -> Option<String> {
        let start = from.find(open)? + open.len();
        let end = from[start..].find(close)? + start;
        Some(xml_text(from[start..end].trim()))
    };
    let settings = between(&text, "<Settings>", "</Settings>").unwrap_or_default();
    let enabled = !settings.contains("<Enabled>false</Enabled>");
    let command = between(&text, "<Command>", "</Command>").unwrap_or_default();
    let arguments = between(&text, "<Arguments>", "</Arguments>").unwrap_or_default();
    TaskXml {
        at_logon: text.contains("<LogonTrigger"),
        enabled,
        command: if arguments.is_empty() {
            command
        } else {
            format!("{command} {arguments}")
        },
        author: between(&text, "<Author>", "</Author>").filter(|a| !a.is_empty()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "fcmd-startup-{name}-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// The user's file wins over the machine's of the same name; a machine's entry is
    /// switched off with a copy in the user's folder, and on again by rewriting that copy;
    /// the machine's own file is never touched.
    #[test]
    fn a_machines_autostart_entry_is_switched_off_with_a_copy_of_the_users_own() {
        let root = temp("xdg");
        let (user, system) = (root.join("user"), root.join("system"));
        std::fs::create_dir_all(&user).unwrap();
        std::fs::create_dir_all(&system).unwrap();
        let machine_text = "[Desktop Entry]\nType=Application\nName=Updater\nName[it]=Aggiornamenti\nExec=/usr/bin/updater --tray\n\n[Desktop Action Quit]\nName=Quit\n";
        std::fs::write(system.join("updater.desktop"), machine_text).unwrap();
        std::fs::write(
            system.join("mine.desktop"),
            "[Desktop Entry]\nName=Machine copy\nExec=/bin/false\n",
        )
        .unwrap();
        std::fs::write(
            user.join("mine.desktop"),
            "[Desktop Entry]\nName=Mine\nExec=/home/me/bin/mine\nX-GNOME-Autostart-enabled=false\n",
        )
        .unwrap();

        let list = desktop_entries(&user, std::slice::from_ref(&system));
        assert_eq!(list.len(), 2, "{list:?}");
        let mine = list
            .iter()
            .find(|s| s.name == "Mine")
            .expect("the user's own");
        assert!(!mine.enabled && !mine.machine);
        let updater = list
            .iter()
            .find(|s| s.name == "Updater")
            .expect("the machine's");
        assert!(updater.enabled && updater.machine);
        assert_eq!(updater.command, "/usr/bin/updater --tray");
        assert_eq!(
            updater.file(),
            Some(system.join("updater.desktop").as_path())
        );

        set_startup(updater, false).unwrap();
        assert_eq!(
            std::fs::read_to_string(system.join("updater.desktop")).unwrap(),
            machine_text,
            "the machine's file was changed"
        );
        let list = desktop_entries(&user, std::slice::from_ref(&system));
        let updater = list.iter().find(|s| s.name == "Updater").unwrap();
        assert!(!updater.enabled && !updater.machine, "{updater:?}");
        assert!(user.join("updater.desktop").exists());

        set_startup(updater, true).unwrap();
        let list = desktop_entries(&user, std::slice::from_ref(&system));
        assert!(list.iter().find(|s| s.name == "Updater").unwrap().enabled);

        // GNOME's own switch follows `Hidden`, so the two never disagree.
        let mine = list.iter().find(|s| s.name == "Mine").unwrap();
        set_startup(mine, true).unwrap();
        let text = std::fs::read_to_string(user.join("mine.desktop")).unwrap();
        assert!(text.contains("Hidden=false"), "{text}");
        assert!(text.contains("X-GNOME-Autostart-enabled=true"), "{text}");

        // A machine's entry is not removed; the user's own copy of one is not either, since
        // removing it would bring the machine's back.
        let list = desktop_entries(&user, std::slice::from_ref(&system));
        let updater = list.iter().find(|s| s.name == "Updater").unwrap();
        assert_eq!(remove_startup(updater), Err(StartupError::NeedsRoot));
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// An entry only the user has is removed with its file.
    #[test]
    fn a_users_own_autostart_entry_is_removed_with_its_file() {
        let root = temp("xdg-rm");
        let user = root.join("user");
        std::fs::create_dir_all(&user).unwrap();
        std::fs::write(user.join("x.desktop"), "[Desktop Entry]\nExec=x\n").unwrap();
        let list = desktop_entries(&user, &[]);
        assert_eq!(list[0].name, "x");
        remove_startup(&list[0]).unwrap();
        assert!(!user.join("x.desktop").exists());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn hidden_is_added_to_the_entry_group_and_not_to_an_action() {
        let text = with_hidden(
            "[Desktop Entry]\nExec=a\n[Desktop Action b]\nExec=b\n",
            true,
        );
        let entry = text.split("[Desktop Action b]").next().unwrap();
        assert!(entry.contains("Hidden=true"), "{text}");
        assert_eq!(text.matches("Hidden=").count(), 1, "{text}");
    }

    #[test]
    fn systemds_user_units_say_what_they_run() {
        let units = user_unit_files(
            "syncthing.service enabled enabled\n\
             pipewire.service static -\n\
             backup.service disabled enabled\n\
             getty@.service enabled enabled\n",
        );
        assert_eq!(
            units,
            vec![
                ("syncthing.service".to_string(), true),
                ("backup.service".to_string(), false)
            ]
        );
        let props = unit_properties(
            "Id=syncthing.service\nDescription=Syncthing\nExecStart={ path=/usr/bin/syncthing ; argv[]=/usr/bin/syncthing serve --no-browser ; ignore_errors=no }\n\nId=backup.service\nDescription=\n",
        );
        assert_eq!(
            exec_line(&props["syncthing.service"]["ExecStart"]),
            "/usr/bin/syncthing serve --no-browser"
        );
        assert!(props.contains_key("backup.service"));
    }

    #[test]
    fn a_launchd_property_list_gives_its_label_its_command_and_whether_it_is_off() {
        let job = parse_plist(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
  <key>Label</key>
  <string>com.example.sync</string>
  <key>ProgramArguments</key>
  <array><string>/Applications/Sync.app/Contents/MacOS/sync</string><string>--quiet &amp; fast</string></array>
  <key>RunAtLoad</key><true/>
  <key>Disabled</key>
  <true/>
</dict></plist>"#,
        );
        assert_eq!(job.label.as_deref(), Some("com.example.sync"));
        assert_eq!(
            job.command,
            "/Applications/Sync.app/Contents/MacOS/sync --quiet & fast"
        );
        assert!(job.disabled);
        assert_eq!(parse_plist("bplist00\u{0}\u{1}"), Plist::default());
    }

    #[test]
    fn task_managers_approval_bytes_read_and_write_as_it_does() {
        assert!(approved(None));
        assert!(approved(Some(&[2, 0, 0, 0])));
        assert!(approved(Some(&[6, 0, 0, 0])));
        assert!(!approved(Some(&[3, 0, 0, 0])));
        assert!(!approved(Some(&[7, 0, 0, 0])));
        let off = approval_bytes(false, 0x0102_0304_0506_0708);
        assert!(!approved(Some(&off)));
        assert_eq!(&off[4..], &0x0102_0304_0506_0708u64.to_le_bytes());
        assert_eq!(
            approval_bytes(true, 99),
            [2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn a_task_file_in_utf16_says_it_runs_at_logon_and_whether_it_is_on() {
        let xml = "<?xml version=\"1.0\" encoding=\"UTF-16\"?>\n<Task><RegistrationInfo><Author>Contoso</Author></RegistrationInfo><Triggers><LogonTrigger><Enabled>true</Enabled></LogonTrigger></Triggers><Settings><Enabled>false</Enabled></Settings><Actions><Exec><Command>C:\\Tools\\agent.exe</Command><Arguments>/tray</Arguments></Exec></Actions></Task>";
        let mut bytes = vec![0xFF, 0xFE];
        for unit in xml.encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        let task = parse_task(&bytes);
        assert!(task.at_logon);
        assert!(!task.enabled);
        assert_eq!(task.command, "C:\\Tools\\agent.exe /tray");
        assert_eq!(task.author.as_deref(), Some("Contoso"));
        let boot =
            parse_task(b"<Task><Triggers><BootTrigger/></Triggers><Settings></Settings></Task>");
        assert!(!boot.at_logon && boot.enabled);
    }

    #[test]
    fn a_name_that_is_not_a_name_never_reaches_a_command_line() {
        for bad in ["", "-x", "a b", "a;b", "$(id)"] {
            assert!(checked_name(bad).is_err(), "{bad}");
        }
        assert!(checked_name("com.apple.x").is_ok());
    }
}
