//! The card `F3` shows for a row of the process list (SPEC 02 §7b).
//!
//! FAR's own process plugin answers `F3` with a page rather than a line: what the thing is,
//! where it came from, who runs it, what it has spent, what it holds open, its whole command
//! line, its environment and every library it has loaded. This builds the same page, out of
//! whatever this platform would tell us, and the caller shows it in the viewer — which is
//! where a page belongs: it scrolls, it searches, and it is the same keys as any other file.
//!
//! Every row is written only where there is an answer. A process of somebody else's tells us
//! less than one of our own — no environment, often no command line — and a card with blanks
//! on it would say the program failed where the system simply refused.

use fcmd_i18n::I18n;
use fcmd_platform::Details;
use std::time::{Duration, SystemTime};

/// The card as text, laid out in sections with the labels in one column.
pub fn card(details: &Details, i18n: &I18n, now: SystemTime) -> String {
    let mut sections: Vec<Vec<(String, String)>> = Vec::new();

    // What it is and whose it is.
    let mut about: Vec<(String, String)> = Vec::new();
    let say = |rows: &mut Vec<(String, String)>, id: &str, value: String| {
        if !value.is_empty() {
            rows.push((i18n.t(id), value));
        }
    };
    say(&mut about, "proc-card-name", details.name.clone());
    say(&mut about, "proc-card-pid", details.pid.to_string());
    if let Some(parent) = details.parent {
        say(&mut about, "proc-card-parent", parent.to_string());
    }
    if let Some(user) = &details.user {
        say(&mut about, "proc-card-user", user.clone());
    }
    if let Some(group) = &details.group {
        say(&mut about, "proc-card-group", group.clone());
    }
    if let Some(sid) = &details.sid {
        say(&mut about, "proc-card-sid", sid.clone());
    }
    if let Some(session) = details.session {
        say(&mut about, "proc-card-session", session.to_string());
    }
    if let Some(state) = &details.state {
        say(&mut about, "proc-card-state", state.clone());
    }
    if let Some(priority) = &details.priority {
        say(&mut about, "proc-card-priority", priority.clone());
    }
    if details.threads > 0 {
        say(&mut about, "proc-card-threads", details.threads.to_string());
    }
    if details.bits > 0 {
        say(
            &mut about,
            "proc-card-bits",
            format!("{}-bit", details.bits),
        );
    }
    if let Some(version) = &details.version {
        say(&mut about, "proc-card-version", version.clone());
    }
    if let Some(description) = &details.description {
        say(&mut about, "proc-card-description", description.clone());
    }
    if let Some(started) = details.started {
        say(
            &mut about,
            "proc-card-started",
            fcmd_ui::format_stamp(started),
        );
        // How long it has been up, which is the figure one is really after: a timestamp has
        // to be subtracted in one's head, and this is the subtraction.
        if let Ok(up) = now.duration_since(started) {
            say(&mut about, "proc-card-uptime", how_long(up));
        }
    }
    sections.push(about);

    // What it has taken and what it has spent.
    let mut cost: Vec<(String, String)> = Vec::new();
    if details.memory > 0 {
        say(
            &mut cost,
            "proc-card-memory",
            fcmd_core::format_size(details.memory),
        );
    }
    if details.private > 0 {
        say(
            &mut cost,
            "proc-card-private",
            fcmd_core::format_size(details.private),
        );
    }
    if details.mapped > 0 {
        say(
            &mut cost,
            "proc-card-mapped",
            fcmd_core::format_size(details.mapped),
        );
    }
    if details.swap > 0 {
        say(
            &mut cost,
            "proc-card-swap",
            fcmd_core::format_size(details.swap),
        );
    }
    if details.faults > 0 {
        say(&mut cost, "proc-card-faults", group_digits(details.faults));
    }
    if let Some(user) = details.cpu_user {
        say(&mut cost, "proc-card-cpu-user", how_long(user));
    }
    if let Some(kernel) = details.cpu_kernel {
        say(&mut cost, "proc-card-cpu-kernel", how_long(kernel));
    }
    if details.handles > 0 {
        say(&mut cost, "proc-card-handles", details.handles.to_string());
    }
    if details.io_read > 0 {
        say(
            &mut cost,
            "proc-card-io-read",
            fcmd_core::format_size(details.io_read),
        );
    }
    if details.io_write > 0 {
        say(
            &mut cost,
            "proc-card-io-write",
            fcmd_core::format_size(details.io_write),
        );
    }
    sections.push(cost);

    // Where it came from and where it is.
    let mut where_: Vec<(String, String)> = Vec::new();
    if let Some(exe) = &details.exe {
        say(&mut where_, "proc-card-exe", exe.clone());
    }
    if let Some(cwd) = &details.cwd {
        say(&mut where_, "proc-card-cwd", cwd.clone());
    }
    if let Some(command) = &details.command {
        say(&mut where_, "proc-card-command", command.clone());
    }
    sections.push(where_);

    let mut out = String::new();
    for section in sections.iter().filter(|s| !s.is_empty()) {
        if !out.is_empty() {
            out.push('\n');
        }
        // The labels of one section line up with each other rather than with the whole card:
        // a long label in the memory block must not push the first block's values across the
        // screen, and each block is read on its own anyway.
        let width = section
            .iter()
            .map(|(label, _)| unicode_width::UnicodeWidthStr::width(label.as_str()))
            .max()
            .unwrap_or(0);
        for (label, value) in section {
            let pad = width - unicode_width::UnicodeWidthStr::width(label.as_str());
            out.push_str(&format!("{label}{:pad$}  {value}\n", ""));
        }
    }

    // The two lists go at the end, each under its own heading and each only where there is
    // one: an empty environment means we were not allowed to read it, and a heading over
    // nothing would read as "this process has no environment", which is never true.
    if !details.env.is_empty() {
        out.push_str(&format!("\n{}\n", i18n.t("proc-card-env")));
        // In the order the system keeps them, which is the order they were set in: sorting
        // them would lose that, and `PATH` is easier to find by searching than by hunting.
        for (name, value) in &details.env {
            out.push_str(&format!("  {name}={value}\n"));
        }
    }
    if !details.modules.is_empty() {
        out.push_str(&format!("\n{}\n", i18n.t("proc-card-modules")));
        for module in &details.modules {
            out.push_str(&format!("  {module}\n"));
        }
    }
    out
}

/// A span in the units somebody reading a card thinks in: seconds under a minute, and days
/// where there are days. It is never more than two units, because the third is noise.
fn how_long(span: Duration) -> String {
    let secs = span.as_secs();
    if secs < 60 {
        // Under a minute the fraction is the interesting part: a process that has spent
        // 0.03 s on the processor and one that has spent 48 s are not the same thing.
        return format!("{:.2} s", span.as_secs_f64());
    }
    let (days, hours, minutes, seconds) = (
        secs / 86_400,
        (secs % 86_400) / 3_600,
        (secs % 3_600) / 60,
        secs % 60,
    );
    if days > 0 {
        format!("{days} d {hours} h")
    } else if hours > 0 {
        format!("{hours} h {minutes} m")
    } else {
        format!("{minutes} m {seconds} s")
    }
}

/// A count with its thousands apart, which is the difference between reading 1 234 567 and
/// counting the digits of 1234567.
fn group_digits(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn details() -> Details {
        Details {
            pid: 1234,
            name: "fcmd".into(),
            parent: Some(1),
            user: Some("mb".into()),
            group: Some("mb".into()),
            state: Some("S (sleeping)".into()),
            priority: Some("20 (nice 0)".into()),
            session: Some(3),
            threads: 7,
            memory: 12 * 1024 * 1024,
            private: 8 * 1024 * 1024,
            mapped: 400 * 1024 * 1024,
            handles: 42,
            faults: 1_234_567,
            cpu_user: Some(Duration::from_millis(1_250)),
            cpu_kernel: Some(Duration::from_millis(300)),
            io_read: 3 * 1024 * 1024,
            bits: 64,
            exe: Some("/usr/bin/fcmd".into()),
            cwd: Some("/home/mb".into()),
            command: Some("fcmd /home/mb /tmp".into()),
            started: Some(SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000)),
            env: vec![("PATH".into(), "/usr/bin".into())],
            modules: vec!["/usr/bin/fcmd".into(), "/lib/libc.so.6".into()],
            ..Details::default()
        }
    }

    #[test]
    fn the_card_says_everything_the_system_answered() {
        let i18n = I18n::new("en");
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_003_600);
        let text = card(&details(), &i18n, now);
        for wanted in [
            "1234",
            "mb",
            "S (sleeping)",
            "20 (nice 0)",
            "64-bit",
            "/usr/bin/fcmd",
            "/home/mb",
            "fcmd /home/mb /tmp",
            "1 234 567",
            "1.25 s",
            "1 h 0 m", // it has been up an hour
            "PATH=/usr/bin",
            "/lib/libc.so.6",
        ] {
            assert!(
                text.contains(wanted),
                "`{wanted}` is not on the card:\n{text}"
            );
        }
        // The two lists are under headings of their own, in the reader's language.
        assert!(text.contains(&i18n.t("proc-card-env")), "{text}");
        assert!(text.contains(&i18n.t("proc-card-modules")), "{text}");
    }

    /// A process that would not answer gets a short card, not a card full of blanks.
    #[test]
    fn what_was_refused_is_left_out() {
        let i18n = I18n::new("en");
        let bare = Details {
            pid: 9,
            name: "kworker".into(),
            ..Details::default()
        };
        let text = card(&bare, &i18n, SystemTime::UNIX_EPOCH);
        assert!(text.contains("kworker") && text.contains('9'), "{text}");
        assert!(!text.contains(&i18n.t("proc-card-env")), "{text}");
        assert!(!text.contains(&i18n.t("proc-card-modules")), "{text}");
        // Nothing it did not say is on it, and no row is left hanging with no value.
        for line in text.lines().filter(|l| !l.is_empty()) {
            assert!(
                line.split_whitespace().count() >= 2,
                "a row with no value: {line:?}"
            );
        }
    }

    #[test]
    fn a_span_is_written_the_way_one_reads_it() {
        assert_eq!(how_long(Duration::from_millis(30)), "0.03 s");
        assert_eq!(how_long(Duration::from_secs(90)), "1 m 30 s");
        assert_eq!(how_long(Duration::from_secs(3 * 3_600 + 60)), "3 h 1 m");
        assert_eq!(how_long(Duration::from_secs(2 * 86_400 + 3_600)), "2 d 1 h");
        assert_eq!(group_digits(7), "7");
        assert_eq!(group_digits(1_234_567), "1 234 567");
    }
}
