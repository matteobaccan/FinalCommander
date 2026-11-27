//! Stored sessions brought in from the programs people had them in before (SPEC 04 §6.7):
//! FAR Manager's FTP plugin, WinSCP and NetBox (WinSCP's sessions inside FAR).
//!
//! What is read is what those programs export, recognised by what it holds rather than by
//! its name:
//!
//! - a `.reg` file — `regedit`'s export, UTF-16 or the older `REGEDIT4` text — with FAR's
//!   `…\Plugins\FTP\Hosts\…` keys (FAR 1.x and 2.x), or WinSCP's and NetBox's
//!   `…\Sessions\<name>` keys;
//! - `WinSCP.ini`, with its `[Sessions\<name>]` sections.
//!
//! Each session becomes a `[[site]]` block whose password is asked for when it is opened.
//! A password FAR's FTP plugin kept with its one-byte XOR cipher is decoded and handed back
//! beside the site, for the caller to put in the encrypted secrets file and nowhere else; a
//! value that does not decode to a plausible password (other FAR builds keep them another
//! way) is left out, and that session's password is asked for. WinSCP's stored passwords
//! are not read. What cannot be a site (a protocol this program has no client for, a
//! session with no host) is listed with why.

use fcmd_config::SiteCfg;

/// What an import found.
#[derive(Debug, Default, PartialEq)]
pub struct Imported {
    pub sites: Vec<SiteCfg>,
    /// Sessions left out, each with why.
    pub skipped: Vec<String>,
    /// What the file was taken for, for the summary: `FAR FTP`, `WinSCP`, `NetBox`.
    pub from: Vec<&'static str>,
    /// Passwords that came with a session: the index of its site in `sites`, and the
    /// password. For the secrets file only.
    pub passwords: Vec<(usize, String)>,
}

/// FAR's FTP plugin's password cipher, undone: the first byte XOR the second, with `0x50`
/// set, is the key every byte after the second is XORed with; the text ends at a NUL.
/// `None` for anything that is not a plausible password.
pub fn far_password(bytes: &[u8]) -> Option<String> {
    if bytes.len() < 3 {
        return None;
    }
    let key = (bytes[0] ^ bytes[1]) | 0x50;
    let plain: Vec<u8> = bytes[2..].iter().map(|b| b ^ key).collect();
    let text = match plain.iter().position(|&b| b == 0) {
        Some(end) => &plain[..end],
        None => &plain[..],
    };
    let fine =
        !text.is_empty() && text.len() <= 128 && text.iter().all(|b| (0x20..0x7f).contains(b));
    fine.then(|| String::from_utf8_lossy(text).into_owned())
}

/// Read `bytes` as one of the formats above.
pub fn sessions(bytes: &[u8]) -> Result<Imported, String> {
    let text = decode(bytes);
    let head = text.trim_start();
    if head.starts_with("Windows Registry Editor") || head.starts_with("REGEDIT4") {
        return Ok(from_registry(&parse_reg(&text)));
    }
    if text.lines().any(|l| l.trim().starts_with("[Sessions\\")) {
        let mut out = Imported::default();
        for (name, values) in parse_ini_sessions(&text) {
            winscp_session(&name, &values, &mut out);
        }
        if !out.sites.is_empty() || !out.skipped.is_empty() {
            out.from.push("WinSCP");
        }
        return Ok(out);
    }
    Err("not a registry export (.reg) or a WinSCP.ini: nothing to import".into())
}

/// UTF-16 with its byte order mark, as `regedit` writes; UTF-8; or the ANSI text of a
/// `REGEDIT4` file, read as Windows-1252 — the host names and users that matter are ASCII.
fn decode(bytes: &[u8]) -> String {
    let utf16 = |le: bool| {
        let units: Vec<u16> = bytes[2..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&c| {
                if le {
                    u16::from_le_bytes(c)
                } else {
                    u16::from_be_bytes(c)
                }
            })
            .collect();
        String::from_utf16_lossy(&units)
    };
    match bytes {
        [0xFF, 0xFE, ..] => utf16(true),
        [0xFE, 0xFF, ..] => utf16(false),
        [0xEF, 0xBB, 0xBF, rest @ ..] => String::from_utf8_lossy(rest).into_owned(),
        _ => match std::str::from_utf8(bytes) {
            Ok(text) => text.to_string(),
            Err(_) => bytes.iter().map(|&b| char::from(b)).collect(),
        },
    }
}

/// A value of a registry export, as far as sessions need: a string or a number.
#[derive(Debug, Clone, PartialEq)]
enum Value {
    Text(String),
    Number(u32),
    /// `hex:…`, the bytes of a binary value.
    Bytes(Vec<u8>),
    Other,
}

/// The keys of a `.reg` file in order, each with its values.
fn parse_reg(text: &str) -> Vec<(String, Vec<(String, Value)>)> {
    let mut keys: Vec<(String, Vec<(String, Value)>)> = Vec::new();
    let mut continuing = false;
    // A `hex:` value whose lines go on: its name and the bytes so far.
    let mut gathering: Option<(String, Vec<u8>)> = None;
    for raw in text.lines() {
        let line = raw.trim_end_matches('\r');
        if continuing {
            continuing = line.ends_with('\\');
            if let Some((_, bytes)) = gathering.as_mut() {
                bytes.extend(hex_bytes(line));
            }
            if !continuing {
                if let (Some((name, bytes)), Some((_, values))) =
                    (gathering.take(), keys.last_mut())
                {
                    values.push((name, Value::Bytes(bytes)));
                }
            }
            continue;
        }
        let trimmed = line.trim();
        if let Some(key) = trimmed.strip_prefix('[').and_then(|k| k.strip_suffix(']')) {
            keys.push((key.to_string(), Vec::new()));
            continue;
        }
        let Some(rest) = trimmed.strip_prefix('"') else {
            if trimmed.starts_with('@') {
                continuing = trimmed.ends_with('\\');
            }
            continue;
        };
        let Some((name, value)) = split_quoted(rest) else {
            continue;
        };
        let value = value.trim_start();
        let Some(value) = value.strip_prefix('=') else {
            continue;
        };
        let parsed = if let Some(text) = value.strip_prefix('"') {
            Value::Text(unescape(text.strip_suffix('"').unwrap_or(text)))
        } else if let Some(hex) = value.strip_prefix("dword:") {
            u32::from_str_radix(hex.trim(), 16)
                .map(Value::Number)
                .unwrap_or(Value::Other)
        } else if let Some(hex) = value.strip_prefix("hex:") {
            if hex.ends_with('\\') {
                continuing = true;
                gathering = Some((name, hex_bytes(hex)));
                continue;
            }
            Value::Bytes(hex_bytes(hex))
        } else {
            continuing = value.ends_with('\\');
            Value::Other
        };
        if let Some((_, values)) = keys.last_mut() {
            values.push((name, parsed));
        }
    }
    keys
}

/// `name"…` → the name, unescaped, and what follows its closing quote.
fn split_quoted(rest: &str) -> Option<(String, &str)> {
    let mut escaped = false;
    for (i, c) in rest.char_indices() {
        match c {
            '\\' if !escaped => escaped = true,
            '"' if !escaped => return Some((unescape(&rest[..i]), &rest[i + 1..])),
            _ => escaped = false,
        }
    }
    None
}

fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next) = chars.next() {
                out.push(next);
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn from_registry(keys: &[(String, Vec<(String, Value)>)]) -> Imported {
    let mut out = Imported::default();
    let mut far = false;
    let mut winscp = false;
    for (key, values) in keys {
        let lower = key.to_ascii_lowercase();
        let text = |name: &str| -> Option<String> {
            values.iter().find_map(|(n, v)| match v {
                Value::Text(t) if n.eq_ignore_ascii_case(name) => Some(t.clone()),
                _ => None,
            })
        };
        if let Some(at) = lower.find("\\plugins\\ftp\\hosts\\") {
            // A folder of FAR's FTP plugin is a key of its own with `Folder` set; the sessions
            // under it keep its name as their group.
            let rest = &key[at + "\\plugins\\ftp\\hosts\\".len()..];
            let folder = values
                .iter()
                .any(|(n, v)| n.eq_ignore_ascii_case("Folder") && *v == Value::Number(1));
            if folder || text("HostName").is_none() {
                continue;
            }
            let group = rest.rsplit_once('\\').map(|(g, _)| g.replace('\\', "/"));
            far = true;
            let password = values.iter().find_map(|(n, v)| match v {
                Value::Bytes(b) if n.eq_ignore_ascii_case("Password") => far_password(b),
                _ => None,
            });
            let before = out.sites.len();
            far_session(
                &text("HostName").unwrap_or_default(),
                text("User"),
                text("Description"),
                group,
                &mut out,
            );
            if let (Some(password), true) = (password, out.sites.len() > before) {
                out.passwords.push((before, password));
            }
        } else if let Some(at) = lower.rfind("\\sessions\\") {
            let name = &key[at + "\\sessions\\".len()..];
            let values: Vec<(String, String)> = values
                .iter()
                .filter_map(|(n, v)| match v {
                    Value::Text(t) => Some((n.clone(), t.clone())),
                    Value::Number(x) => Some((n.clone(), x.to_string())),
                    Value::Bytes(_) | Value::Other => None,
                })
                .collect();
            if values
                .iter()
                .any(|(n, _)| n.eq_ignore_ascii_case("HostName"))
            {
                winscp = true;
                winscp_session(name, &values, &mut out);
            }
        }
    }
    if far {
        out.from.push("FAR FTP");
    }
    if winscp {
        out.from.push(
            if keys
                .iter()
                .any(|(k, _)| k.to_ascii_lowercase().contains("netbox"))
            {
                "NetBox"
            } else {
                "WinSCP"
            },
        );
    }
    out
}

/// One session of FAR's FTP plugin. Its host field takes several shapes, all seen in real
/// exports: `host`, `host:port`, `ftp://host/path`, and a label of one's own in front —
/// `OLD:host`, `OLD:ftp://host` — which becomes the group.
fn far_session(
    host_field: &str,
    user: Option<String>,
    description: Option<String>,
    group: Option<String>,
    out: &mut Imported,
) {
    let mut rest = host_field.trim().to_string();
    let mut label = None;
    let mut scheme = "ftp".to_string();
    if let Some((left, right)) = rest.clone().split_once("://") {
        match left.rsplit_once(':') {
            Some((tag, proto)) => {
                label = Some(tag.to_string());
                scheme = proto.to_ascii_lowercase();
            }
            None => scheme = left.to_ascii_lowercase(),
        }
        rest = right.to_string();
    } else if let Some((left, right)) = rest.clone().split_once(':') {
        // `host:21` is a port; `OLD:host` is a label.
        let port_part = right.split('/').next().unwrap_or("");
        if !(port_part.chars().all(|c| c.is_ascii_digit()) && !port_part.is_empty()) {
            label = Some(left.to_string());
            rest = right.to_string();
        }
    }
    let (authority, path) = match rest.split_once('/') {
        Some((a, p)) => (a.to_string(), format!("/{p}")),
        None => (rest.clone(), String::new()),
    };
    // A user written into the address, `user@host`, wins over an empty user field.
    let (user_in_host, hostport) = match authority.rsplit_once('@') {
        Some((u, h)) => (Some(u.to_string()), h.to_string()),
        None => (None, authority),
    };
    let (host, port) = match hostport.rsplit_once(':') {
        Some((h, p)) if p.parse::<u16>().is_ok() => (h.to_string(), p.parse().ok()),
        _ => (hostport, None),
    };
    if host.is_empty() {
        out.skipped.push(format!("{host_field}: no host in it"));
        return;
    }
    let scheme = match scheme.as_str() {
        "ftp" | "ftps" | "sftp" => scheme,
        other => {
            out.skipped.push(format!(
                "{host_field}: {other} is not a protocol this program speaks"
            ));
            return;
        }
    };
    let user = user.filter(|u| !u.trim().is_empty()).or(user_in_host);
    let label_group = label.filter(|l| !l.trim().is_empty());
    let id = description
        .filter(|d| !d.trim().is_empty())
        .unwrap_or_else(|| host.clone());
    out.sites.push(SiteCfg {
        id,
        scheme,
        host,
        port,
        user,
        auth: Some("ask".into()),
        start_dir: (!path.is_empty() && path != "/").then_some(path),
        group: group.or(label_group),
        ..SiteCfg::default()
    });
}

/// `[Sessions\name]` sections of a `WinSCP.ini`, each with its `key=value` lines.
fn parse_ini_sessions(text: &str) -> Vec<(String, Vec<(String, String)>)> {
    let mut out: Vec<(String, Vec<(String, String)>)> = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        let line = line.trim();
        if let Some(section) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            inside = match section.strip_prefix("Sessions\\") {
                Some(name) => {
                    out.push((name.to_string(), Vec::new()));
                    true
                }
                None => false,
            };
            continue;
        }
        if !inside {
            continue;
        }
        if let (Some((k, v)), Some((_, values))) = (line.split_once('='), out.last_mut()) {
            values.push((k.trim().to_string(), v.trim().to_string()));
        }
    }
    out
}

/// `00,4f,a1,\` → the bytes; what is not a pair of hex digits is passed over.
fn hex_bytes(line: &str) -> Vec<u8> {
    line.split(',')
        .filter_map(|b| u8::from_str_radix(b.trim().trim_end_matches('\\').trim(), 16).ok())
        .collect()
}

/// `%20` and its kin, as WinSCP writes a session's name.
fn url_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let (hi, lo) = (bytes[i + 1], bytes[i + 2]);
            if hi.is_ascii_hexdigit() && lo.is_ascii_hexdigit() {
                let digit = |d: u8| (d as char).to_digit(16).unwrap_or(0) as u8;
                out.push(digit(hi) * 16 + digit(lo));
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// One WinSCP (or NetBox) session.
fn winscp_session(name: &str, values: &[(String, String)], out: &mut Imported) {
    let name = url_decode(name);
    if name.eq_ignore_ascii_case("Default Settings") {
        return;
    }
    let get = |key: &str| -> Option<String> {
        values
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| url_decode(v))
            .filter(|v| !v.is_empty())
    };
    let number = |key: &str| get(key).and_then(|v| v.parse::<u32>().ok());
    let Some(host) = get("HostName") else {
        out.skipped.push(format!("{name}: no host in it"));
        return;
    };
    // WinSCP's numbers: 0 SCP, 1 SFTP (falling back to SCP), 2 SFTP, 5 FTP, 6 WebDAV, 7 S3.
    let tls = number("Ftps").unwrap_or(0) != 0;
    let scheme = match number("FSProtocol").unwrap_or(2) {
        0..=2 => "sftp",
        5 if tls => "ftps",
        5 => "ftp",
        6 if tls => "davs",
        6 => "dav",
        7 => {
            out.skipped.push(format!(
                "{name}: an S3 session — object storage is set up in [[s3]] blocks"
            ));
            return;
        }
        other => {
            out.skipped.push(format!(
                "{name}: WinSCP protocol {other} is not one this program speaks"
            ));
            return;
        }
    };
    let (group, id) = match name.rsplit_once('/') {
        Some((group, id)) => (Some(group.to_string()), id.to_string()),
        None => (None, name.clone()),
    };
    let auth = match get("PublicKeyFile") {
        Some(key) if scheme == "sftp" => format!("key:{key}"),
        _ if scheme == "sftp" => "agent".to_string(),
        _ => "ask".to_string(),
    };
    out.sites.push(SiteCfg {
        id,
        scheme: scheme.to_string(),
        host,
        port: number("PortNumber").and_then(|p| u16::try_from(p).ok()),
        user: get("UserName"),
        auth: Some(auth),
        start_dir: get("RemoteDirectory"),
        group,
        ..SiteCfg::default()
    });
}

/// Add `found` to `sites`, leaving out what is already there (the same protocol, host, port
/// and user) and giving a name already taken a number. Returns how many were added and how
/// many were there already.
pub fn merge(sites: &mut Vec<SiteCfg>, found: Vec<SiteCfg>) -> (usize, usize) {
    let same = |a: &SiteCfg, b: &SiteCfg| {
        a.scheme.eq_ignore_ascii_case(&b.scheme)
            && a.host.eq_ignore_ascii_case(&b.host)
            && a.port == b.port
            && a.user == b.user
    };
    let (mut added, mut there) = (0, 0);
    for mut site in found {
        if sites.iter().any(|s| same(s, &site)) {
            there += 1;
            continue;
        }
        if sites.iter().any(|s| s.id == site.id) {
            let base = site.id.clone();
            let mut n = 2;
            while sites.iter().any(|s| s.id == format!("{base} ({n})")) {
                n += 1;
            }
            site.id = format!("{base} ({n})");
        }
        sites.push(site);
        added += 1;
    }
    (added, there)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf16(text: &str) -> Vec<u8> {
        let mut out = vec![0xFF, 0xFE];
        for unit in text.encode_utf16() {
            out.extend_from_slice(&unit.to_le_bytes());
        }
        out
    }

    /// FAR 1.5's FTP plugin, as `regedit` exports it: every shape of host field met in a
    /// real export, an encrypted password over several lines that is not brought across.
    #[test]
    fn far_ftp_hosts_become_sites_without_their_passwords() {
        let reg = "Windows Registry Editor Version 5.00\r\n\r\n\
[HKEY_CURRENT_USER\\SOFTWARE\\Far\\Plugins\\FTP\\Hosts]\r\n\r\n\
[HKEY_CURRENT_USER\\SOFTWARE\\Far\\Plugins\\FTP\\Hosts\\Item0]\r\n\
\"HostName\"=\"OLD:old.example.net\"\r\n\
\"User\"=\"anna\"\r\n\
\"Password\"=hex:00,01,02,\\\r\n  03,04,05\r\n\
\"PassiveMode\"=dword:00000001\r\n\
\"Description\"=\"\"\r\n\
\"Folder\"=dword:00000000\r\n\r\n\
[HKEY_CURRENT_USER\\SOFTWARE\\Far\\Plugins\\FTP\\Hosts\\Item1]\r\n\
\"HostName\"=\"files.example.com:2121\"\r\n\
\"User\"=\"bob\"\r\n\
\"Description\"=\"The \\\"files\\\" box\"\r\n\r\n\
[HKEY_CURRENT_USER\\SOFTWARE\\Far\\Plugins\\FTP\\Hosts\\Item2]\r\n\
\"HostName\"=\"ftp://mirror.example.org/pub/linux\"\r\n\
\"User\"=\"\"\r\n\r\n\
[HKEY_CURRENT_USER\\SOFTWARE\\Far\\Plugins\\FTP\\Hosts\\Item3]\r\n\
\"HostName\"=\"OLD:ftp://gone.example.it/\"\r\n\
\"User\"=\"carla\"\r\n\r\n\
[HKEY_CURRENT_USER\\SOFTWARE\\Far\\Plugins\\FTP\\Hosts\\Item4]\r\n\
\"HostName\"=\"\"\r\n";
        let found = sessions(&utf16(reg)).unwrap();
        assert_eq!(found.from, ["FAR FTP"]);
        let hosts: Vec<_> = found
            .sites
            .iter()
            .map(|s| {
                (
                    s.scheme.as_str(),
                    s.host.as_str(),
                    s.port,
                    s.user.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            hosts,
            [
                ("ftp", "old.example.net", None, Some("anna")),
                ("ftp", "files.example.com", Some(2121), Some("bob")),
                ("ftp", "mirror.example.org", None, None),
                ("ftp", "gone.example.it", None, Some("carla")),
            ]
        );
        assert_eq!(found.sites[0].group.as_deref(), Some("OLD"), "the label");
        assert_eq!(found.sites[1].id, "The \"files\" box", "the description");
        assert_eq!(found.sites[2].start_dir.as_deref(), Some("/pub/linux"));
        assert!(found.sites.iter().all(|s| s.auth.as_deref() == Some("ask")));
        assert_eq!(found.skipped.len(), 1, "{:?}", found.skipped);
    }

    /// WinSCP's own `WinSCP.ini`: protocols, folders, keys, the default session left out.
    #[test]
    fn a_winscp_ini_becomes_sites() {
        let ini = "[Configuration\\Interface]\nLanguage=0410\n\n\
[Sessions\\Default%20Settings]\nHostName=\n\n\
[Sessions\\Work/web%20server]\nHostName=web.example.com\nUserName=deploy\nPortNumber=2222\n\
FSProtocol=2\nPublicKeyFile=C:%5Ckeys%5Cid.ppk\nRemoteDirectory=/var/www\n\n\
[Sessions\\nas]\nHostName=192.168.1.5\nUserName=admin\nFSProtocol=5\nFtps=3\n\n\
[Sessions\\bucket]\nHostName=s3.amazonaws.com\nFSProtocol=7\n";
        let found = sessions(ini.as_bytes()).unwrap();
        assert_eq!(found.from, ["WinSCP"]);
        assert_eq!(found.sites.len(), 2);
        let web = &found.sites[0];
        assert_eq!(
            (web.id.as_str(), web.group.as_deref(), web.scheme.as_str()),
            ("web server", Some("Work"), "sftp")
        );
        assert_eq!(web.port, Some(2222));
        assert_eq!(web.auth.as_deref(), Some("key:C:\\keys\\id.ppk"));
        assert_eq!(web.start_dir.as_deref(), Some("/var/www"));
        assert_eq!(found.sites[1].scheme, "ftps");
        assert_eq!(found.skipped.len(), 1, "S3: {:?}", found.skipped);
    }

    /// NetBox keeps WinSCP's own keys, in the registry.
    #[test]
    fn netbox_sessions_in_a_registry_export_become_sites() {
        let reg = "REGEDIT4\r\n\r\n\
[HKEY_CURRENT_USER\\Software\\Far2\\Plugins\\NetBox\\Sessions\\home%2Fpi]\r\n\
\"HostName\"=\"raspberrypi.local\"\r\n\
\"UserName\"=\"pi\"\r\n\
\"PortNumber\"=dword:00000016\r\n\
\"FSProtocol\"=dword:00000002\r\n";
        let found = sessions(reg.as_bytes()).unwrap();
        assert_eq!(found.from, ["NetBox"]);
        let pi = &found.sites[0];
        assert_eq!((pi.id.as_str(), pi.group.as_deref()), ("pi", Some("home")));
        assert_eq!(pi.port, Some(22));
        assert_eq!(pi.scheme, "sftp");
    }

    #[test]
    fn what_is_already_there_is_not_added_twice_and_names_stay_apart() {
        let mut sites = vec![SiteCfg {
            id: "box".into(),
            scheme: "ftp".into(),
            host: "box.example".into(),
            user: Some("me".into()),
            ..SiteCfg::default()
        }];
        let again = sites[0].clone();
        let other = SiteCfg {
            host: "other.example".into(),
            ..again.clone()
        };
        assert_eq!(merge(&mut sites, vec![again, other]), (1, 1));
        assert_eq!(sites[1].id, "box (2)");
    }

    /// FAR's cipher on a password it kept; a value it is not, which is left out; and a
    /// `hex:` value over two lines, read whole.
    #[test]
    fn a_far_password_is_decoded_and_anything_else_is_not() {
        let kept = [
            0xfe, 0x80, 0x2d, 0x0b, 0x0e, 0x1b, 0x0c, 0x2e, 0x1f, 0x0d, 0x0d, 0x09, 0x11, 0x0c,
            0x1a,
        ];
        assert_eq!(far_password(&kept).as_deref(), Some("SuperPassword"));
        assert_eq!(far_password(&[0xff, 0x89, 0x9c, 0x03, 0xe1, 0x00]), None);

        let reg = "Windows Registry Editor Version 5.00\r\n\r\n\
[HKEY_CURRENT_USER\\Software\\Far\\Plugins\\FTP\\Hosts\\Item0]\r\n\
\"HostName\"=\"ftp.example.com\"\r\n\
\"User\"=\"me\"\r\n\
\"Password\"=hex:fe,80,2d,0b,0e,1b,0c,2e,\\\r\n  1f,0d,0d,09,11,0c,1a\r\n";
        let found = sessions(&utf16(reg)).unwrap();
        assert_eq!(found.sites.len(), 1);
        assert_eq!(found.passwords, [(0, "SuperPassword".to_string())]);
    }

    #[test]
    fn something_else_is_refused() {
        assert!(sessions(b"just some text").is_err());
    }
}
