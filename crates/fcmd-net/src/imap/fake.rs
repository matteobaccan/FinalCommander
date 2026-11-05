//! An IMAP server, in process, for the tests of the client beside it: the handful of
//! commands the client sends, over a real socket, answered from a fixed set of mailboxes.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

type Boxes = Arc<Vec<(String, Vec<(u32, String)>)>>;

pub struct FakeImap {
    pub port: u16,
    /// Every command line received, tag and all.
    pub seen: Arc<Mutex<Vec<String>>>,
}

impl FakeImap {
    /// Mailboxes by name (`/` is the separator), each with its messages as (uid, raw text).
    pub async fn start(boxes: Vec<(&str, Vec<(u32, &str)>)>) -> FakeImap {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("a port");
        let addr: SocketAddr = listener.local_addr().expect("the address");
        let boxes: Boxes = Arc::new(
            boxes
                .into_iter()
                .map(|(n, m)| {
                    (
                        n.to_string(),
                        m.into_iter().map(|(u, t)| (u, t.to_string())).collect(),
                    )
                })
                .collect(),
        );
        let seen = Arc::new(Mutex::new(Vec::new()));
        let (b, s) = (boxes.clone(), seen.clone());
        tokio::spawn(async move {
            while let Ok((socket, _)) = listener.accept().await {
                let (b, s) = (b.clone(), s.clone());
                tokio::spawn(async move {
                    let _ = serve(socket, b, s).await;
                });
            }
        });
        FakeImap {
            port: addr.port(),
            seen,
        }
    }
}

async fn serve(
    socket: TcpStream,
    boxes: Boxes,
    seen: Arc<Mutex<Vec<String>>>,
) -> std::io::Result<()> {
    let (read, mut write) = socket.into_split();
    let mut lines = BufReader::new(read);
    write.write_all(b"* OK fake IMAP ready\r\n").await?;
    let mut selected: Option<usize> = None;
    loop {
        let mut line = String::new();
        if lines.read_line(&mut line).await? == 0 {
            return Ok(());
        }
        let line = line.trim_end().to_string();
        seen.lock().unwrap().push(line.clone());
        let (tag, rest) = line.split_once(' ').unwrap_or((&line, ""));
        let upper = rest.to_ascii_uppercase();
        let mut out = String::new();
        if upper.starts_with("LOGIN") {
            if rest.contains("\"secret\"") {
                out.push_str(&format!("{tag} OK logged in\r\n"));
            } else {
                out.push_str(&format!(
                    "{tag} NO [AUTHENTICATIONFAILED] wrong password\r\n"
                ));
            }
        } else if upper == "LIST \"\" \"\"" {
            out.push_str("* LIST (\\Noselect) \"/\" \"\"\r\n");
            out.push_str(&format!("{tag} OK done\r\n"));
        } else if upper.starts_with("LIST") {
            for (name, _) in boxes.iter() {
                out.push_str(&format!("* LIST (\\HasNoChildren) \"/\" \"{name}\"\r\n"));
            }
            out.push_str(&format!("{tag} OK done\r\n"));
        } else if upper.starts_with("EXAMINE") {
            let name = rest[8..].trim().trim_matches('"');
            selected = boxes.iter().position(|(n, _)| n == name);
            match selected {
                Some(at) => {
                    out.push_str(&format!("* {} EXISTS\r\n", boxes[at].1.len()));
                    out.push_str(&format!("{tag} OK [READ-ONLY] examined\r\n"));
                }
                None => out.push_str(&format!("{tag} NO no such mailbox\r\n")),
            }
        } else if upper.starts_with("UID FETCH 1:*") {
            let messages = selected.map(|at| &boxes[at].1);
            for (i, (uid, text)) in messages.into_iter().flatten().enumerate() {
                let header: String =
                    text.split("\r\n\r\n").next().unwrap_or("").to_string() + "\r\n\r\n";
                out.push_str(&format!(
                    "* {} FETCH (UID {uid} RFC822.SIZE {} INTERNALDATE \"05-Nov-2026 10:00:00 +0000\" BODY[HEADER.FIELDS (SUBJECT)] {{{}}}\r\n{header})\r\n",
                    i + 1,
                    text.len(),
                    header.len()
                ));
            }
            out.push_str(&format!("{tag} OK fetched\r\n"));
        } else if upper.starts_with("UID FETCH") {
            let uid: u32 = rest
                .split_whitespace()
                .nth(2)
                .and_then(|u| u.parse().ok())
                .unwrap_or(0);
            let found = selected.and_then(|at| boxes[at].1.iter().find(|(u, _)| *u == uid));
            if let Some((uid, text)) = found {
                out.push_str(&format!(
                    "* 1 FETCH (UID {uid} BODY[] {{{}}}\r\n{text})\r\n",
                    text.len()
                ));
            }
            out.push_str(&format!("{tag} OK fetched\r\n"));
        } else if upper.starts_with("LOGOUT") {
            write
                .write_all(format!("* BYE\r\n{tag} OK bye\r\n").as_bytes())
                .await?;
            return Ok(());
        } else {
            out.push_str(&format!("{tag} BAD unknown\r\n"));
        }
        write.write_all(out.as_bytes()).await?;
    }
}
