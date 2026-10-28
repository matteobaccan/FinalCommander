//! What the machine runs in the background, and the three things one does to it (SPEC 02 §7d).
//!
//! A file manager is where one already is when a service has to be looked at — the log is in
//! a panel, the configuration file is in the other one — and FAR has had a services panel for
//! as long as it has had a process list. This is the same list: what is there, what state it
//! is in, and start, stop or restart.
//!
//! Every platform keeps it somewhere different and all three are asked the way their own
//! tools ask:
//!
//! * **Linux** has systemd, and systemd answers `systemctl`. The unit list is parsed from
//!   `systemctl list-units`, which is the documented machine-readable form of it; there is no
//!   D-Bus dependency here for the same reason there is no GUI toolkit.
//! * **Windows** has the Service Control Manager, which is an API and is called as one.
//! * **macOS** has `launchctl`, whose list is three columns and whose `kickstart -k` is a
//!   restart.
//!
//! **Nothing here runs as anybody else.** Stopping a system service wants root, and this does
//! not reach for `sudo`, `pkexec` or any other way of asking for it: it runs the command as
//! the user running the program and, when that is refused, says plainly that the thing needs
//! root rather than appearing to have worked. A file manager that silently escalated would be
//! a file manager nobody could audit.

use std::fmt;

// The names the rest of the workspace knows these by: `service::State` reads well in here and
// badly at a call site, where `ServiceState` says which state is meant.
pub use Action as ServiceAction;
pub use Error as ServiceError;
pub use State as ServiceState;

/// What state a service is in, in the three words every platform agrees on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum State {
    /// It is up.
    Running,
    /// It is there and down.
    Stopped,
    /// It tried and failed, which is the state one opens this panel to find.
    Failed,
    /// Starting, stopping, or something this platform calls its own name for. The default,
    /// because a row we have not read yet is not a row that is running.
    #[default]
    Other,
}

impl State {
    pub fn as_str(self) -> &'static str {
        match self {
            State::Running => "running",
            State::Stopped => "stopped",
            State::Failed => "failed",
            State::Other => "other",
        }
    }
}

/// One service, as the panel shows it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Service {
    /// What the system calls it, and what the three actions are asked for by: a unit name on
    /// Linux, a service name on Windows, a label on macOS.
    pub id: String,
    /// What a human calls it, where the system keeps a second name. Empty where it does not.
    pub display: String,
    pub state: State,
    /// What it says about itself, where it says anything.
    pub description: String,
    /// Its process, where it has one and the system will say which.
    pub pid: Option<u32>,
}

/// What to do to one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Start,
    Stop,
    Restart,
}

impl Action {
    pub fn as_str(self) -> &'static str {
        match self {
            Action::Start => "start",
            Action::Stop => "stop",
            Action::Restart => "restart",
        }
    }
}

/// Why an action did not happen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// This platform has no services this program knows how to ask about, or the tool that
    /// answers for them is not installed.
    Unsupported,
    /// The system refused: the action wants root, or administrator.
    NeedsRoot,
    /// Anything else, in the system's own words.
    Failed(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Unsupported => write!(f, "no service manager here"),
            Error::NeedsRoot => write!(f, "needs root"),
            Error::Failed(what) => write!(f, "{what}"),
        }
    }
}

/// Everything the machine runs in the background, as far as this user may see.
///
/// The order is the system's; the panel sorts it, as it sorts a directory.
pub fn services() -> Vec<Service> {
    #[cfg(target_os = "linux")]
    {
        systemd()
    }
    #[cfg(windows)]
    {
        super::windows::services()
    }
    #[cfg(target_os = "macos")]
    {
        launchd()
    }
    #[cfg(not(any(target_os = "linux", windows, target_os = "macos")))]
    {
        Vec::new()
    }
}

/// Start, stop or restart one. The name is the one the listing gave.
pub fn control(id: &str, action: Action) -> Result<(), Error> {
    // A name is handed to a command line, so it is checked before it goes anywhere near one:
    // a unit name is letters, digits and a handful of punctuation, and nothing here has to
    // accept a name with a space, a quote or a semicolon in it.
    if id.is_empty() || !id.chars().all(ok_in_name) {
        return Err(Error::Failed(format!("`{id}` is not a service name")));
    }
    #[cfg(target_os = "linux")]
    {
        run(
            "systemctl",
            &[action.as_str(), id, "--no-ask-password", "--no-pager"],
        )
    }
    #[cfg(windows)]
    {
        super::windows::control_service(id, action)
    }
    #[cfg(target_os = "macos")]
    {
        match action {
            // `launchctl` has no restart of its own: `kickstart -k` kills and starts again,
            // which is what a restart is there.
            Action::Restart => run("launchctl", &["kickstart", "-k", &format!("system/{id}")]),
            Action::Start => run("launchctl", &["start", id]),
            Action::Stop => run("launchctl", &["stop", id]),
        }
    }
    #[cfg(not(any(target_os = "linux", windows, target_os = "macos")))]
    {
        let _ = action;
        Err(Error::Unsupported)
    }
}

/// The characters a service name may have. Deliberately narrow: this string is handed to a
/// program, and the narrowest set that covers every real name is the one to allow.
fn ok_in_name(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '@' | ':' | '\\' | '/')
}

/// Run a tool and read its answer. Anything on `stderr` about permission is reported as what
/// it is, because "needs root" is an answer and "failed" is not.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn run(program: &str, args: &[&str]) -> Result<(), Error> {
    use std::process::Command;

    let out = match Command::new(program).args(args).output() {
        Ok(out) => out,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(Error::Unsupported),
        Err(e) => return Err(Error::Failed(e.to_string())),
    };
    if out.status.success() {
        return Ok(());
    }
    let said = String::from_utf8_lossy(&out.stderr);
    let said = said.trim();
    let lower = said.to_ascii_lowercase();
    if lower.contains("access denied")
        || lower.contains("permission denied")
        || lower.contains("authentication")
        || lower.contains("interactive authentication required")
        || lower.contains("must be root")
        || lower.contains("not permitted")
    {
        return Err(Error::NeedsRoot);
    }
    Err(Error::Failed(if said.is_empty() {
        format!("{program} refused")
    } else {
        said.lines().next().unwrap_or(said).to_string()
    }))
}

/// systemd's own list, in the form it offers for reading by a program: no column headings, no
/// pager, and the columns separated by runs of spaces.
#[cfg(target_os = "linux")]
fn systemd() -> Vec<Service> {
    use std::process::Command;

    let Ok(out) = Command::new("systemctl")
        .args([
            "list-units",
            "--type=service",
            "--all",
            "--no-legend",
            "--plain",
            "--no-pager",
            "--full",
        ])
        .output()
    else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut services = Vec::new();
    for line in text.lines() {
        // `UNIT LOAD ACTIVE SUB DESCRIPTION`, and a unit that is gone is marked with a bullet
        // in front of its name.
        let line = line.trim_start_matches(['●', '*', ' ']);
        let mut parts = line.split_whitespace();
        let (Some(unit), Some(_load), Some(active), Some(sub)) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        if !unit.ends_with(".service") {
            continue;
        }
        let description = parts.collect::<Vec<_>>().join(" ");
        services.push(Service {
            id: unit.to_string(),
            display: String::new(),
            state: match (active, sub) {
                ("active", "running") => State::Running,
                ("active", _) => State::Other,
                ("failed", _) => State::Failed,
                ("inactive", _) => State::Stopped,
                _ => State::Other,
            },
            description,
            pid: None,
        });
    }
    services
}

/// launchd's list: `PID Status Label`, with a dash where there is no process.
#[cfg(target_os = "macos")]
fn launchd() -> Vec<Service> {
    use std::process::Command;

    let Ok(out) = Command::new("launchctl").arg("list").output() else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut services = Vec::new();
    for line in text.lines().skip(1) {
        let mut parts = line.split_whitespace();
        let (Some(pid), Some(status), Some(label)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        let pid = pid.parse::<u32>().ok();
        services.push(Service {
            id: label.to_string(),
            display: String::new(),
            // launchd says the last exit status rather than a state: a job with a process is
            // running, one that left a non-zero status behind it failed, and the rest are
            // simply not running.
            state: match (pid, status) {
                (Some(_), _) => State::Running,
                (None, "0") => State::Stopped,
                (None, _) => State::Failed,
            },
            description: String::new(),
            pid,
        });
    }
    services
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A name that is not one never reaches a command line.
    #[test]
    fn a_name_that_is_not_a_name_is_refused_before_anything_runs() {
        for bad in ["", "nginx; rm -rf /", "a b", "`id`", "$(id)", "a'b"] {
            assert!(
                matches!(control(bad, Action::Stop), Err(Error::Failed(_))),
                "`{bad}` was not refused"
            );
        }
    }

    /// The list is whatever this machine has — a container often has no service manager at
    /// all — so what is checked is that every row that came back is a row worth drawing.
    #[test]
    fn every_service_listed_has_a_name() {
        for service in services() {
            assert!(!service.id.is_empty());
            assert!(!service.id.contains(char::is_whitespace), "{service:?}");
        }
    }
}
