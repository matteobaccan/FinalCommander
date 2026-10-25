//! Reading what an FTP server says a directory holds.
//!
//! There are two answers and they are not alike. `MLSD` (RFC 3659) is machine-readable: each
//! line is a set of `fact=value;` pairs and then the name, with the type, the size and the
//! time said plainly and in UTC. `LIST` is whatever the server's own `ls` prints — a different
//! shape on a Unix, on a Windows and on a mainframe, and with a time field that drops either
//! the clock or the year depending on how old the file is.
//!
//! So `MLSD` is asked for first and this module is written around it; `LIST` is the fallback,
//! parsed for the two layouts that actually turn up: Unix `ls -l` and IIS's DOS form. Both are
//! pure functions over one line, which is what lets them be tested properly with no server
//! anywhere near them.

use scmd_core::EntryKind;

use crate::ops::RemoteEntry;

/// One line of an `MLSD` answer: `type=file;size=11;modify=20260101120000; readme.md`.
///
/// Facts are case-insensitive, as the RFC says, and the name is everything after the single
/// space that follows the last fact — a name may hold spaces, semicolons and equals signs, so
/// it is taken as the rest of the line rather than split out of it.
pub fn mlsd_line(line: &str) -> Option<RemoteEntry> {
    let line = line.trim_end_matches(['\r', '\n']);
    let (facts, name) = line.split_once(' ')?;
    let name = name.trim_start();
    if name.is_empty() || name == "." || name == ".." {
        return None;
    }
    let mut kind = EntryKind::File;
    let mut size = 0u64;
    let mut mtime = None;
    let mut mode = None;
    for fact in facts.split(';').filter(|f| !f.is_empty()) {
        let Some((key, value)) = fact.split_once('=') else {
            continue;
        };
        match key.to_ascii_lowercase().as_str() {
            "type" => {
                let value = value.to_ascii_lowercase();
                kind = match value.as_str() {
                    "dir" | "cdir" | "pdir" => EntryKind::Dir,
                    "file" => EntryKind::File,
                    // `OS.unix=slink:/target` is the usual spelling of a symlink.
                    other if other.starts_with("os.unix=slink") || other.starts_with("slink") => {
                        EntryKind::Symlink
                    }
                    _ => EntryKind::Other,
                };
            }
            "size" | "sizd" => size = value.parse().unwrap_or(0),
            "modify" => mtime = mlsd_time(value),
            // `UNIX.mode=0644`: octal, however many digits it is written with.
            "unix.mode" | "unix.modez" => mode = u32::from_str_radix(value, 8).ok(),
            _ => {}
        }
    }
    Some(RemoteEntry {
        name: name.to_string(),
        kind,
        size: if kind == EntryKind::Dir { 0 } else { size },
        mtime,
        mode,
        link_target: None,
    })
}

/// `YYYYMMDDHHMMSS` in UTC, which is what RFC 3659 says a `modify` fact is. A fraction of a
/// second may follow a `.` and is dropped: the panel shows minutes.
pub fn mlsd_time(text: &str) -> Option<u32> {
    let digits = text.split('.').next()?;
    if digits.len() < 14 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let num = |from: usize, to: usize| digits[from..to].parse::<i64>().ok();
    crate::clock::seconds_from_utc(
        num(0, 4)?,
        num(4, 6)?,
        num(6, 8)?,
        num(8, 10)?,
        num(10, 12)?,
        num(12, 14)?,
    )
}

/// One line of a `LIST` answer, in either of the two layouts that turn up:
///
/// ```text
/// -rw-r--r--   1 mb  users     11 Jan  1 12:00 readme.md
/// 01-01-26  12:00PM       <DIR>          pictures
/// ```
///
/// The time is deliberately **not** read. A Unix `LIST` gives either a clock or a year, never
/// both, and never a time zone; a date wrong by a year, or by the server's offset from ours,
/// is worse than no date, because sorting and "only files newer than these" both believe it.
/// A time comes from `MLSD`; a server without it gets rows with no time rather than rows with
/// a plausible lie.
pub fn list_line(line: &str) -> Option<RemoteEntry> {
    let line = line.trim_end_matches(['\r', '\n']);
    if line.trim().is_empty() {
        return None;
    }
    // A Unix listing starts with the permission string. `total 8` and anything else falls
    // through to the DOS form, which refuses it.
    let first = line.as_bytes()[0];
    if line.len() > 10 && matches!(first, b'-' | b'd' | b'l' | b'b' | b'c' | b'p' | b's') {
        return unix_line(line);
    }
    dos_line(line)
}

fn unix_line(line: &str) -> Option<RemoteEntry> {
    let mut fields = line.split_whitespace();
    let perms = fields.next()?;
    if perms.len() < 10 {
        return None;
    }
    let kind = match perms.as_bytes()[0] {
        b'd' => EntryKind::Dir,
        b'l' => EntryKind::Symlink,
        b'-' => EntryKind::File,
        _ => EntryKind::Other,
    };
    let mode = mode_of(&perms[1..]);
    let _links = fields.next()?;
    let _owner = fields.next()?;
    // Some servers leave the group column out, so the size is the next field that is a number
    // — looked for over the next two fields and no further, so a name is never read as one.
    let mut size = None;
    for (seen, field) in fields.by_ref().enumerate() {
        if let Ok(n) = field.parse::<u64>() {
            size = Some(n);
            break;
        }
        if seen >= 1 {
            return None;
        }
    }
    let size = size?;
    // Then the month, the day and the clock-or-year; the name is the rest of the line.
    let (_month, _day, _when) = (fields.next()?, fields.next()?, fields.next()?);
    let rest = fields.collect::<Vec<_>>().join(" ");
    if rest.is_empty() {
        return None;
    }
    // `link -> target`, which only a symlink has.
    let (name, link_target) = match (kind, rest.split_once(" -> ")) {
        (EntryKind::Symlink, Some((name, target))) => (name.to_string(), Some(target.to_string())),
        _ => (rest, None),
    };
    if name == "." || name == ".." {
        return None;
    }
    Some(RemoteEntry {
        name,
        kind,
        size: if kind == EntryKind::Dir { 0 } else { size },
        mtime: None,
        mode,
        link_target,
    })
}

fn dos_line(line: &str) -> Option<RemoteEntry> {
    let mut fields = line.split_whitespace();
    let _date = fields.next()?;
    let _time = fields.next()?;
    let third = fields.next()?;
    let (kind, size) = if third.eq_ignore_ascii_case("<DIR>") {
        (EntryKind::Dir, 0)
    } else {
        (EntryKind::File, third.replace(',', "").parse().ok()?)
    };
    let name = fields.collect::<Vec<_>>().join(" ");
    if name.is_empty() || name == "." || name == ".." {
        return None;
    }
    Some(RemoteEntry {
        name,
        kind,
        size,
        mtime: None,
        // A Windows server has no Unix mode, and inventing 0644 would be a lie the attribute
        // dialog would then offer to change.
        mode: None,
        link_target: None,
    })
}

/// `rw-r--r--` → `0o644`. The setuid, setgid and sticky spellings are read as the execute bit
/// they sit on: lowercase `s` and `t` mean it is set, uppercase `S` and `T` mean it is not.
fn mode_of(perms: &str) -> Option<u32> {
    let bytes = perms.as_bytes();
    if bytes.len() < 9 {
        return None;
    }
    let mut mode = 0;
    for (i, byte) in bytes[..9].iter().enumerate() {
        let bit = match i % 3 {
            0 => 0o4,
            1 => 0o2,
            _ => 0o1,
        };
        let on = match byte {
            b'-' | b'S' | b'T' => false,
            b'r' | b'w' | b'x' | b's' | b't' => true,
            _ => return None,
        };
        if on {
            mode |= bit << (6 - 3 * (i / 3));
        }
    }
    Some(mode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mlsd_says_the_type_the_size_and_the_time() {
        let entry = mlsd_line("type=file;size=11;modify=20260101120000; readme.md").unwrap();
        assert_eq!(entry.name, "readme.md");
        assert_eq!(entry.kind, EntryKind::File);
        assert_eq!(entry.size, 11);
        // 2026-01-01 12:00:00 UTC.
        assert_eq!(entry.mtime, Some(1_767_268_800));

        let dir = mlsd_line("type=dir;sizd=4096;modify=20251224235959; pictures").unwrap();
        assert_eq!(dir.kind, EntryKind::Dir);
        assert_eq!(dir.name, "pictures");
        assert_eq!(dir.size, 0, "a directory's own size is not shown");

        // Facts are case-insensitive, the mode is octal, a fraction of a second is dropped,
        // and a name may hold spaces.
        let odd =
            mlsd_line("Type=FILE;Size=3;UNIX.mode=0755;Modify=20260101120000.123; my notes.txt")
                .unwrap();
        assert_eq!(odd.name, "my notes.txt");
        assert_eq!(odd.mode, Some(0o755));
        assert_eq!(odd.mtime, Some(1_767_268_800));

        let link = mlsd_line("type=OS.unix=slink:/var/log/latest; now").unwrap();
        assert_eq!(link.kind, EntryKind::Symlink);

        // `.` and `..` are the panel's own business, and rubbish is not an entry.
        assert!(mlsd_line("type=cdir;modify=20260101120000; .").is_none());
        assert!(mlsd_line("type=pdir; ..").is_none());
        assert!(mlsd_line("").is_none());
        assert!(mlsd_line("type=file;size=1;").is_none());
    }

    #[test]
    fn a_bad_mlsd_time_is_no_time_rather_than_a_wrong_one() {
        assert_eq!(mlsd_time("20260101120000"), Some(1_767_268_800));
        assert_eq!(mlsd_time("19700101000000"), Some(0));
        assert_eq!(mlsd_time("2026010112000"), None, "one digit short");
        assert_eq!(mlsd_time("2026AA01120000"), None);
        assert_eq!(mlsd_time("20261301120000"), None, "month 13");
        // Before the epoch there is nothing a `u32` of seconds can say.
        assert_eq!(mlsd_time("19600101000000"), None);
    }

    #[test]
    fn a_unix_listing_is_read_the_way_ls_prints_it() {
        let file = list_line("-rw-r--r--   1 mb  users     11 Jan  1 12:00 readme.md").unwrap();
        assert_eq!(file.name, "readme.md");
        assert_eq!(file.kind, EntryKind::File);
        assert_eq!(file.size, 11);
        assert_eq!(file.mode, Some(0o644));
        assert_eq!(file.mtime, None, "a LIST clock is not to be trusted");

        let dir = list_line("drwxr-xr-x   2 mb  users   4096 Dec 24  2025 pictures").unwrap();
        assert_eq!(dir.kind, EntryKind::Dir);
        assert_eq!(dir.size, 0);
        assert_eq!(dir.mode, Some(0o755));

        let link =
            list_line("lrwxrwxrwx   1 mb  users      7 Jan  1 12:00 now -> latest.log").unwrap();
        assert_eq!(link.kind, EntryKind::Symlink);
        assert_eq!(link.name, "now");
        assert_eq!(link.link_target.as_deref(), Some("latest.log"));

        // A name with spaces survives, and the group column may be missing.
        let spaced =
            list_line("-rw-r--r--   1 mb        3 Jan  1 12:00 my holiday notes.txt").unwrap();
        assert_eq!(spaced.name, "my holiday notes.txt");
        assert_eq!(spaced.size, 3);

        // The sticky bit on a world-writable directory, as /tmp has it.
        let sticky = list_line("drwxrwxrwt   8 root root  4096 Jan  1 12:00 tmp").unwrap();
        assert_eq!(sticky.mode, Some(0o777));

        assert!(list_line("total 8").is_none());
        assert!(list_line("").is_none());
    }

    #[test]
    fn a_dos_listing_is_read_the_way_iis_prints_it() {
        let dir = list_line("01-01-26  12:00PM       <DIR>          pictures").unwrap();
        assert_eq!(dir.kind, EntryKind::Dir);
        assert_eq!(dir.name, "pictures");
        assert_eq!(dir.mode, None, "a Windows server has no Unix mode to show");

        let file = list_line("01-01-26  12:00PM                 1,234 report.txt").unwrap();
        assert_eq!(file.kind, EntryKind::File);
        assert_eq!(file.size, 1234);
        assert_eq!(file.name, "report.txt");
    }
}
