//! What is listening and what is connected (SPEC 02 §7c).
//!
//! FAR has no socket panel of its own, but the question a file manager is asked next to "what
//! is running" is "what has that port open", and the answer lives in the same place: a table
//! the system keeps, and a map from each row of it back to the process that holds it. Linux
//! keeps both as files; Windows has one call per protocol that gives the owning pid with the
//! row. Nothing here closes a socket, for the same reason the process panel kills nothing.
//!
//! Every field is best-effort, as in the process list. The owning process of somebody else's
//! socket is not something this user may always be told, and a row nobody would name is left
//! without a name rather than given a guess.

/// One socket, as the panel shows it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Socket {
    /// `tcp`, `tcp6`, `udp`, `udp6`.
    pub proto: String,
    /// The address on this machine, as `host:port`.
    pub local: String,
    /// The other end, empty for a socket that is only listening.
    pub remote: String,
    /// `LISTEN`, `ESTABLISHED`, … and empty for UDP, which has no state to be in.
    pub state: String,
    /// Who holds it, where the system will say.
    pub pid: Option<u32>,
    /// That process's name, where it could be read.
    pub process: Option<String>,
}

/// Every socket this user is allowed to see.
///
/// The order is the system's; the panel sorts it, as it sorts a directory.
pub fn sockets() -> Vec<Socket> {
    #[cfg(target_os = "linux")]
    {
        linux()
    }
    #[cfg(windows)]
    {
        super::windows::sockets()
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        // The BSDs and macOS keep it behind the same kind of `sysctl` as the process list,
        // with the same problem (`docs/BACKLOG.md`).
        Vec::new()
    }
}

/// Linux writes the tables as text, one file per protocol, and the link from a socket to its
/// process is the other way round: every process's open files include `socket:[12345]`, where
/// the number is the inode the table's last column holds. So the tables are read first and
/// the processes once, rather than once per socket.
#[cfg(target_os = "linux")]
fn linux() -> Vec<Socket> {
    let owners = socket_owners();
    let mut out = Vec::new();
    for (proto, path, six) in [
        ("tcp", "/proc/net/tcp", false),
        ("tcp6", "/proc/net/tcp6", true),
        ("udp", "/proc/net/udp", false),
        ("udp6", "/proc/net/udp6", true),
    ] {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue; // a kernel built without it, or a container that hides it
        };
        for line in text.lines().skip(1) {
            let fields: Vec<&str> = line.split_whitespace().collect();
            // sl, local, remote, state, queues, timers, retransmits, uid, timeout, inode
            if fields.len() < 10 {
                continue;
            }
            let Some(local) = address(fields[1], six) else {
                continue;
            };
            let remote = address(fields[2], six).unwrap_or_default();
            let tcp = proto.starts_with("tcp");
            let state = if tcp {
                state_name(fields[3]).to_string()
            } else {
                String::new()
            };
            // A connected socket's other end is worth showing; a listener's is `0.0.0.0:0`,
            // which says nothing, so it is left empty.
            let remote = if remote.ends_with(":0") {
                String::new()
            } else {
                remote
            };
            let inode: u64 = fields[9].parse().unwrap_or(0);
            let (pid, process) = match owners.get(&inode) {
                Some((pid, name)) => (Some(*pid), Some(name.clone())),
                None => (None, None),
            };
            out.push(Socket {
                proto: proto.to_string(),
                local,
                remote,
                state,
                pid,
                process,
            });
        }
    }
    out
}

/// Socket inode → the process holding it, by reading every process's open files.
///
/// A process somebody else owns will not show us its files, which is not an error: its
/// sockets are in the table all the same and simply go unnamed.
#[cfg(target_os = "linux")]
fn socket_owners() -> std::collections::HashMap<u64, (u32, String)> {
    let mut out = std::collections::HashMap::new();
    let Ok(dir) = std::fs::read_dir("/proc") else {
        return out;
    };
    for entry in dir.flatten() {
        let name = entry.file_name();
        let Some(pid) = name.to_str().and_then(|s| s.parse::<u32>().ok()) else {
            continue;
        };
        let Ok(fds) = std::fs::read_dir(entry.path().join("fd")) else {
            continue; // not ours to look at, or it ended
        };
        let mut process: Option<String> = None;
        for fd in fds.flatten() {
            let Ok(target) = std::fs::read_link(fd.path()) else {
                continue;
            };
            let Some(inode) = target
                .to_str()
                .and_then(|t| t.strip_prefix("socket:["))
                .and_then(|t| t.strip_suffix(']'))
                .and_then(|n| n.parse::<u64>().ok())
            else {
                continue;
            };
            // Read the name once, and only for a process that holds a socket at all.
            let name = match &process {
                Some(name) => name.clone(),
                None => {
                    let read = std::fs::read_to_string(entry.path().join("comm"))
                        .map(|s| s.trim().to_string())
                        .unwrap_or_default();
                    process = Some(read.clone());
                    read
                }
            };
            out.insert(inode, (pid, name));
        }
    }
    out
}

/// `0100007F:1F90` as `127.0.0.1:8080`, and the same for the sixteen-byte form.
///
/// The address is written as 32-bit words in the machine's own order, which is why each group
/// of eight hex digits is read as a number and then taken apart again.
#[cfg(target_os = "linux")]
fn address(field: &str, six: bool) -> Option<String> {
    let (host, port) = field.rsplit_once(':')?;
    let port = u16::from_str_radix(port, 16).ok()?;
    if six {
        if host.len() != 32 {
            return None;
        }
        let mut bytes = [0u8; 16];
        for (i, chunk) in host.as_bytes().chunks(8).enumerate() {
            let word = u32::from_str_radix(std::str::from_utf8(chunk).ok()?, 16).ok()?;
            bytes[i * 4..i * 4 + 4].copy_from_slice(&word.to_le_bytes());
        }
        let ip = std::net::Ipv6Addr::from(bytes);
        Some(format!("[{ip}]:{port}"))
    } else {
        if host.len() != 8 {
            return None;
        }
        let word = u32::from_str_radix(host, 16).ok()?;
        let ip = std::net::Ipv4Addr::from(word.to_le_bytes());
        Some(format!("{ip}:{port}"))
    }
}

/// The two hex digits the kernel writes for a TCP socket's state.
#[cfg(target_os = "linux")]
fn state_name(code: &str) -> &'static str {
    match code {
        "01" => "ESTABLISHED",
        "02" => "SYN_SENT",
        "03" => "SYN_RECV",
        "04" => "FIN_WAIT1",
        "05" => "FIN_WAIT2",
        "06" => "TIME_WAIT",
        "07" => "CLOSE",
        "08" => "CLOSE_WAIT",
        "09" => "LAST_ACK",
        "0A" => "LISTEN",
        "0B" => "CLOSING",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(target_os = "linux")]
    fn an_address_is_read_the_way_the_kernel_writes_it() {
        use super::address;

        assert_eq!(
            address("0100007F:1F90", false).as_deref(),
            Some("127.0.0.1:8080")
        );
        assert_eq!(
            address("00000000:0016", false).as_deref(),
            Some("0.0.0.0:22")
        );
        // The sixteen-byte form, and the one that matters most: the all-zero address a
        // listener binds, which is `[::]`.
        assert_eq!(
            address("00000000000000000000000000000000:0050", true).as_deref(),
            Some("[::]:80")
        );
        // A loopback in the sixth form: the last word holds the one.
        assert_eq!(
            address("00000000000000000000000001000000:0050", true).as_deref(),
            Some("[::1]:80")
        );
        assert_eq!(address("nonsense", false), None);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn the_table_is_read_and_the_listeners_have_no_other_end() {
        let all = super::sockets();
        // A machine with no socket at all is possible in a sealed container, so the shape is
        // what is checked rather than the count.
        for socket in &all {
            assert!(!socket.proto.is_empty());
            assert!(socket.local.contains(':'), "{socket:?}");
            if socket.state == "LISTEN" {
                assert!(socket.remote.is_empty(), "{socket:?}");
            }
        }
    }
}
