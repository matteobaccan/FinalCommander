//! `fcmd --web-terminal ADDR`: the program in a browser's page, over this machine's loopback
//! (SPEC 11 §2a).
//!
//! A phone has no terminal to run a terminal program in, and the Android app is a page in a
//! web view, so the terminal comes from here: a small HTTP server on `127.0.0.1` that serves
//! one page — a terminal written in JavaScript (xterm.js, MIT), compiled into this binary
//! like the rest — and joins it over a WebSocket to a pseudo-terminal on which `fcmd` itself
//! runs. On a desktop the same option puts the program in a browser tab.
//!
//! What keeps it this machine's own:
//! - it listens on a loopback address and nothing else: an address that leaves the machine
//!   is refused, because what is behind the socket is a shell's worth of power;
//! - the address it prints carries a token, chosen at random for the run, and the page and
//!   the WebSocket both want it: another program on the same machine cannot open a session
//!   without having been told it;
//! - a request whose `Host` is not a loopback name is refused, so a web page elsewhere
//!   cannot reach the server through a name it controls that resolves here.
//!
//! The protocol on the socket is the smallest that works: binary frames carry bytes, both
//! ways, as the terminal reads and writes them; a text frame `r COLS ROWS` says the page
//! changed size. HTTP and RFC 6455 are written here, in a few hundred lines, rather than
//! taken from a server framework: this serves one page to one browser on one machine.

use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// The files the page is made of, compiled in.
const INDEX: &str = include_str!("../assets/webterm/index.html");
const XTERM_JS: &str = include_str!("../assets/webterm/xterm.js");
const XTERM_CSS: &str = include_str!("../assets/webterm/xterm.css");
const FIT_JS: &str = include_str!("../assets/webterm/addon-fit.js");

/// RFC 6455 §1.3: what the key is joined with before it is hashed.
const WS_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// The largest frame accepted from the page. What it sends is keystrokes and pastes.
const MAX_FRAME: u64 = 1 << 20;

/// The largest request head read before giving up on a client.
const MAX_HEAD: usize = 16 * 1024;

/// What the program in the terminal is started with.
#[derive(Debug, Clone)]
pub struct Program {
    pub path: PathBuf,
    pub args: Vec<String>,
    /// Added to the environment this process has.
    pub env: Vec<(String, String)>,
}

/// A listening server: its address, the token it wants, and what it runs.
pub struct Server {
    listener: TcpListener,
    token: String,
    program: Program,
}

impl Server {
    /// Listen on `addr` — a port alone means `127.0.0.1` — which must be a loopback address.
    pub fn bind(addr: &str, program: Program) -> io::Result<Server> {
        let text = if addr.parse::<u16>().is_ok() {
            format!("127.0.0.1:{addr}")
        } else {
            addr.to_string()
        };
        let addrs: Vec<SocketAddr> = text.to_socket_addrs()?.collect();
        let Some(addr) = addrs.iter().find(|a| a.ip().is_loopback()).copied() else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "{text} is not a loopback address: the web terminal listens on this \
                     machine alone (127.0.0.1 or ::1)"
                ),
            ));
        };
        Ok(Server {
            listener: TcpListener::bind(addr)?,
            token: token(),
            program,
        })
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    /// The address to open: the page, with the token.
    pub fn url(&self) -> io::Result<String> {
        Ok(format!("http://{}/?t={}", self.local_addr()?, self.token))
    }

    /// Answer clients until the process ends, each on a thread of its own.
    pub fn serve(self) -> io::Result<()> {
        let token = Arc::new(self.token);
        let program = Arc::new(self.program);
        for stream in self.listener.incoming() {
            let Ok(stream) = stream else { continue };
            let token = Arc::clone(&token);
            let program = Arc::clone(&program);
            std::thread::spawn(move || {
                let _ = handle(stream, &token, &program);
            });
        }
        Ok(())
    }
}

/// `fcmd --web-terminal ADDR`: listen, say where, and serve until killed.
pub fn run(addr: &str, program: Program) -> io::Result<()> {
    let server = Server::bind(addr, program)?;
    // One line, alone on standard output, so that whatever started this — the Android app
    // — can read where to point its web view.
    let mut out = io::stdout();
    writeln!(out, "{}", server.url()?)?;
    out.flush()?;
    server.serve()
}

/// The arguments this run was given, less the web terminal's own: what the program in the
/// terminal is started with, so `--config` and the rest reach it.
pub fn passed_on(args: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut out = Vec::new();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == "--web-terminal" {
            args.next();
        } else if !arg.starts_with("--web-terminal=") {
            out.push(arg);
        }
    }
    out
}

/// A token for the run: 128 bits from the system's random source.
fn token() -> String {
    let mut bytes = [0u8; 16];
    let filled = std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut bytes))
        .is_ok();
    if !filled {
        // No `/dev/urandom` (Windows): the standard library's hasher keys are random per
        // process, and two of them hashing the clock make 128 bits nobody can guess.
        use std::hash::{BuildHasher, Hasher};
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        for half in 0..2 {
            let mut h = std::collections::hash_map::RandomState::new().build_hasher();
            h.write_u128(now);
            h.write_usize(half);
            bytes[half * 8..half * 8 + 8].copy_from_slice(&h.finish().to_le_bytes());
        }
    }
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Compare without stopping at the first difference, so the time a refusal takes says
/// nothing about how much of the token was right.
fn same(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |acc, (x, y)| acc | (x ^ y))
            == 0
}

/// One request's head: the path, the query and the headers (names in lower case).
#[derive(Debug, Default)]
struct Request {
    method: String,
    path: String,
    query: HashMap<String, String>,
    headers: HashMap<String, String>,
}

fn read_request(reader: &mut impl BufRead) -> io::Result<Request> {
    let mut head = Vec::new();
    loop {
        let mut line = Vec::new();
        let n = reader.read_until(b'\n', &mut line)?;
        if n == 0 {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        head.extend_from_slice(&line);
        if head.len() > MAX_HEAD {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "request head too long",
            ));
        }
        if line == b"\r\n" || line == b"\n" {
            break;
        }
    }
    let text = String::from_utf8_lossy(&head);
    let mut lines = text.lines();
    let first = lines.next().unwrap_or_default();
    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let target = parts.next().unwrap_or("/");
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let query = query
        .split('&')
        .filter_map(|pair| {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            (!k.is_empty()).then(|| (k.to_string(), unescape(v)))
        })
        .collect();
    let headers = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
        .collect();
    Ok(Request {
        method,
        path: path.to_string(),
        query,
        headers,
    })
}

/// `%xx` in a query value, which is all a token or a number can carry.
fn unescape(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(b) = u8::from_str_radix(&text[i + 1..i + 3], 16) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(if bytes[i] == b'+' { b' ' } else { bytes[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Is `host` (a `Host` header) this machine, by a name that cannot be anybody else's?
fn loopback_host(host: &str) -> bool {
    let name = if let Some(rest) = host.strip_prefix('[') {
        rest.split(']').next().unwrap_or_default()
    } else {
        host.rsplit_once(':').map_or(host, |(name, _)| name)
    };
    name.eq_ignore_ascii_case("localhost")
        || name
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
}

fn respond(stream: &mut TcpStream, status: &str, kind: &str, body: &[u8]) -> io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\n\
         Cache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\n\
         Referrer-Policy: no-referrer\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)?;
    stream.flush()
}

fn handle(mut stream: TcpStream, token: &str, program: &Program) -> io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let request = read_request(&mut reader)?;
    let host = request
        .headers
        .get("host")
        .map(String::as_str)
        .unwrap_or("");
    if request.method != "GET" || !loopback_host(host) {
        return respond(&mut stream, "403 Forbidden", "text/plain", b"Forbidden\n");
    }
    let has_token = request.query.get("t").is_some_and(|t| same(t, token));
    match request.path.as_str() {
        "/" if has_token => respond(
            &mut stream,
            "200 OK",
            "text/html; charset=utf-8",
            INDEX.as_bytes(),
        ),
        "/" => respond(
            &mut stream,
            "403 Forbidden",
            "text/plain; charset=utf-8",
            b"This page needs the address fcmd printed when it started, token and all.\n",
        ),
        "/xterm.js" => respond(
            &mut stream,
            "200 OK",
            "text/javascript",
            XTERM_JS.as_bytes(),
        ),
        "/addon-fit.js" => respond(&mut stream, "200 OK", "text/javascript", FIT_JS.as_bytes()),
        "/xterm.css" => respond(&mut stream, "200 OK", "text/css", XTERM_CSS.as_bytes()),
        "/ws" if has_token => {
            let upgrade = request
                .headers
                .get("upgrade")
                .is_some_and(|u| u.eq_ignore_ascii_case("websocket"));
            let Some(key) = request.headers.get("sec-websocket-key").filter(|_| upgrade) else {
                return respond(
                    &mut stream,
                    "400 Bad Request",
                    "text/plain",
                    b"WebSocket only\n",
                );
            };
            write!(
                stream,
                "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\n\
                 Connection: Upgrade\r\nSec-WebSocket-Accept: {}\r\n\r\n",
                accept_key(key)
            )?;
            stream.flush()?;
            let size = |name: &str, default: u16| {
                request
                    .query
                    .get(name)
                    .and_then(|v| v.parse::<u16>().ok())
                    .map_or(default, |v| v.clamp(2, 1000))
            };
            session(stream, reader, program, size("c", 80), size("r", 24))
        }
        "/ws" => respond(&mut stream, "403 Forbidden", "text/plain", b"Forbidden\n"),
        _ => respond(&mut stream, "404 Not Found", "text/plain", b"Not found\n"),
    }
}

/// RFC 6455 §4.2.2: the answer to a key.
fn accept_key(key: &str) -> String {
    use sha1::{Digest, Sha1};
    let digest = Sha1::digest(format!("{key}{WS_GUID}").as_bytes());
    fcmd_tui::base64(&digest)
}

/// One frame: whether it ends a message, its opcode, and its payload (unmasked).
#[derive(Debug, PartialEq, Eq)]
struct Frame {
    fin: bool,
    opcode: u8,
    payload: Vec<u8>,
}

fn read_frame(r: &mut impl Read) -> io::Result<Frame> {
    let mut head = [0u8; 2];
    r.read_exact(&mut head)?;
    let fin = head[0] & 0x80 != 0;
    let opcode = head[0] & 0x0f;
    let masked = head[1] & 0x80 != 0;
    let mut len = u64::from(head[1] & 0x7f);
    if len == 126 {
        let mut b = [0u8; 2];
        r.read_exact(&mut b)?;
        len = u64::from(u16::from_be_bytes(b));
    } else if len == 127 {
        let mut b = [0u8; 8];
        r.read_exact(&mut b)?;
        len = u64::from_be_bytes(b);
    }
    if len > MAX_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "frame too large",
        ));
    }
    // A client's frames are masked (§5.1); one that is not is not a browser.
    if !masked {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "unmasked client frame",
        ));
    }
    let mut mask = [0u8; 4];
    r.read_exact(&mut mask)?;
    let mut payload = vec![0u8; len as usize];
    r.read_exact(&mut payload)?;
    for (i, b) in payload.iter_mut().enumerate() {
        *b ^= mask[i % 4];
    }
    Ok(Frame {
        fin,
        opcode,
        payload,
    })
}

/// A server's frame: never masked, always whole.
fn frame(opcode: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len() + 10);
    out.push(0x80 | opcode);
    match payload.len() {
        n if n < 126 => out.push(n as u8),
        n if n <= 0xffff => {
            out.push(126);
            out.extend_from_slice(&(n as u16).to_be_bytes());
        }
        n => {
            out.push(127);
            out.extend_from_slice(&(n as u64).to_be_bytes());
        }
    }
    out.extend_from_slice(payload);
    out
}

/// A close frame with a status and a reason the page shows.
fn close_frame(code: u16, reason: &str) -> Vec<u8> {
    let mut payload = code.to_be_bytes().to_vec();
    // A control frame's payload is at most 125 bytes (§5.5).
    let reason = &reason.as_bytes()[..reason.len().min(123)];
    payload.extend_from_slice(reason);
    frame(0x8, &payload)
}

/// Run the program on a pseudo-terminal and pass bytes between it and the socket until
/// either side is done.
fn session(
    stream: TcpStream,
    mut reader: BufReader<TcpStream>,
    program: &Program,
    cols: u16,
    rows: u16,
) -> io::Result<()> {
    let out = Arc::new(Mutex::new(stream));
    let send = |bytes: &[u8]| -> io::Result<()> {
        let mut s = out.lock().unwrap_or_else(|e| e.into_inner());
        s.write_all(bytes)?;
        s.flush()
    };
    let env: Vec<(&str, &str)> = [("TERM", "xterm-256color"), ("COLORTERM", "truecolor")]
        .into_iter()
        .chain(program.env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .collect();
    let spawned = match fcmd_platform::pty::spawn(
        &program.path.to_string_lossy(),
        &program.args,
        None::<&Path>,
        &env,
        cols,
        rows,
    ) {
        Ok(s) => s,
        Err(e) => {
            let _ = send(&close_frame(1011, &format!("fcmd could not start: {e}")));
            return Err(e);
        }
    };
    let pty = Arc::new(Mutex::new(spawned.pty));
    let mut writer = spawned.writer;

    // The program's output, to the page, until it has nothing more to say.
    let pump = {
        let out = Arc::clone(&out);
        let mut from = spawned.reader;
        std::thread::spawn(move || {
            let mut buf = [0u8; 16 * 1024];
            loop {
                match from.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        let mut s = out.lock().unwrap_or_else(|e| e.into_inner());
                        if s.write_all(&frame(0x2, &buf[..n])).is_err() {
                            return;
                        }
                    }
                }
            }
            let mut s = out.lock().unwrap_or_else(|e| e.into_inner());
            let _ = s.write_all(&close_frame(1000, "FinalCommander has ended."));
            let _ = s.flush();
            let _ = s.shutdown(std::net::Shutdown::Both);
        })
    };

    // The page's keys, to the program; a message may come in pieces (§5.4).
    let mut message: Vec<u8> = Vec::new();
    let mut text = false;
    while let Ok(f) = read_frame(&mut reader) {
        match f.opcode {
            0x0..=0x2 => {
                if f.opcode != 0x0 {
                    message.clear();
                    text = f.opcode == 0x1;
                }
                message.extend_from_slice(&f.payload);
                if !f.fin {
                    continue;
                }
                if text {
                    let line = String::from_utf8_lossy(&message);
                    let mut words = line.split_whitespace();
                    if words.next() == Some("r") {
                        let mut n = words.filter_map(|w| w.parse::<u16>().ok());
                        if let (Some(c), Some(r)) = (n.next(), n.next()) {
                            let p = pty.lock().unwrap_or_else(|e| e.into_inner());
                            let _ = p.resize(c.clamp(2, 1000), r.clamp(2, 1000));
                        }
                    }
                } else if writer
                    .write_all(&message)
                    .and_then(|_| writer.flush())
                    .is_err()
                {
                    break;
                }
                message.clear();
            }
            0x8 => {
                let _ = send(&frame(0x8, &f.payload));
                break;
            }
            0x9 => {
                let _ = send(&frame(0xA, &f.payload));
            }
            _ => {}
        }
    }
    // The page went away: so does the program it was showing.
    pty.lock().unwrap_or_else(|e| e.into_inner()).kill();
    let _ = pump.join();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A client's frame, masked as a browser masks it.
    fn client_frame(opcode: u8, payload: &[u8]) -> Vec<u8> {
        let mask = [0x37, 0xfa, 0x21, 0x3d];
        let mut out = vec![0x80 | opcode];
        assert!(payload.len() < 126);
        out.push(0x80 | payload.len() as u8);
        out.extend_from_slice(&mask);
        out.extend(payload.iter().enumerate().map(|(i, b)| b ^ mask[i % 4]));
        out
    }

    /// RFC 6455's own example of a key and its answer.
    #[test]
    fn the_handshake_answers_the_rfcs_example() {
        assert_eq!(
            accept_key("dGhlIHNhbXBsZSBub25jZQ=="),
            "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
        );
    }

    #[test]
    fn a_masked_frame_reads_back_and_long_lengths_are_written_long() {
        let bytes = client_frame(0x2, b"Hello");
        let f = read_frame(&mut &bytes[..]).unwrap();
        assert_eq!(
            f,
            Frame {
                fin: true,
                opcode: 2,
                payload: b"Hello".to_vec()
            }
        );
        // An unmasked one is refused: only a browser talks to this.
        assert!(read_frame(&mut &frame(0x2, b"x")[..]).is_err());
        assert_eq!(&frame(0x2, &[0; 200])[..4], &[0x82, 126, 0, 200]);
        assert_eq!(&frame(0x2, &vec![0; 70_000])[..2], &[0x82, 127]);
    }

    #[test]
    fn only_a_loopback_address_and_a_loopback_host_are_taken() {
        let program = Program {
            path: "sh".into(),
            args: Vec::new(),
            env: Vec::new(),
        };
        assert!(Server::bind("0.0.0.0:0", program.clone()).is_err());
        let server = Server::bind("0", program).unwrap();
        assert!(server.local_addr().unwrap().ip().is_loopback());
        assert!(server.url().unwrap().contains("/?t="));
        for host in ["127.0.0.1:8080", "localhost:1", "[::1]:9", "LOCALHOST"] {
            assert!(loopback_host(host), "{host}");
        }
        for host in ["", "example.com:80", "192.168.1.2:80", "127.0.0.1.evil.com"] {
            assert!(!loopback_host(host), "{host}");
        }
    }

    #[test]
    fn the_web_terminals_own_arguments_are_not_passed_on() {
        let args = [
            "--web-terminal",
            "0",
            "--config",
            "x.toml",
            "--web-terminal=1",
            "/tmp",
        ]
        .map(String::from);
        assert_eq!(passed_on(args), ["--config", "x.toml", "/tmp"]);
    }

    fn get(addr: SocketAddr, target: &str, host: &str) -> String {
        let mut s = TcpStream::connect(addr).unwrap();
        write!(s, "GET {target} HTTP/1.1\r\nHost: {host}\r\n\r\n").unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).unwrap();
        out
    }

    /// The whole way: the page with the token and not without it, then a WebSocket on
    /// which a program on a pseudo-terminal reads what is typed and writes back, and whose
    /// end closes the socket with the reason the page shows.
    #[cfg(unix)]
    #[test]
    fn a_session_carries_bytes_both_ways_and_says_when_the_program_ends() {
        let program = Program {
            path: "/bin/sh".into(),
            args: vec![
                "-c".into(),
                "printf ready; read x; printf \"got:$x\"".into(),
            ],
            env: Vec::new(),
        };
        let server = Server::bind("127.0.0.1:0", program).unwrap();
        let addr = server.local_addr().unwrap();
        let token = server.token.clone();
        std::thread::spawn(move || server.serve());
        let host = addr.to_string();

        let page = get(addr, &format!("/?t={token}"), &host);
        assert!(page.starts_with("HTTP/1.1 200"), "{page}");
        assert!(page.contains("xterm.js"));
        assert!(get(addr, "/?t=wrong", &host).starts_with("HTTP/1.1 403"));
        assert!(get(addr, &format!("/?t={token}"), "evil.example:80").starts_with("HTTP/1.1 403"));
        assert!(get(addr, "/xterm.js", &host).starts_with("HTTP/1.1 200"));

        let mut s = TcpStream::connect(addr).unwrap();
        write!(
            s,
            "GET /ws?t={token}&c=100&r=30 HTTP/1.1\r\nHost: {host}\r\nUpgrade: websocket\r\n\
             Connection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
             Sec-WebSocket-Version: 13\r\n\r\n"
        )
        .unwrap();
        let mut r = BufReader::new(s.try_clone().unwrap());
        let mut head = String::new();
        loop {
            let mut line = String::new();
            r.read_line(&mut line).unwrap();
            head.push_str(&line);
            if line == "\r\n" {
                break;
            }
        }
        assert!(head.starts_with("HTTP/1.1 101"), "{head}");
        assert!(head.contains("s3pPLMBiTxaQ9kYGzzhZRbK+xOo="));

        // Server frames are unmasked: read them by hand.
        let next = |r: &mut BufReader<TcpStream>| -> (u8, Vec<u8>) {
            let mut h = [0u8; 2];
            r.read_exact(&mut h).unwrap();
            let mut len = usize::from(h[1] & 0x7f);
            if len == 126 {
                let mut b = [0u8; 2];
                r.read_exact(&mut b).unwrap();
                len = usize::from(u16::from_be_bytes(b));
            }
            let mut p = vec![0u8; len];
            r.read_exact(&mut p).unwrap();
            (h[0] & 0x0f, p)
        };
        let mut seen = Vec::new();
        while !String::from_utf8_lossy(&seen).contains("ready") {
            let (op, p) = next(&mut r);
            assert_eq!(op, 2);
            seen.extend(p);
        }
        s.write_all(&client_frame(0x1, b"r 120 40")).unwrap();
        s.write_all(&client_frame(0x2, b"abc\n")).unwrap();
        let mut seen = Vec::new();
        let reason = loop {
            let (op, p) = next(&mut r);
            if op == 0x8 {
                break String::from_utf8_lossy(&p[2..]).into_owned();
            }
            seen.extend(p);
        };
        assert!(
            String::from_utf8_lossy(&seen).contains("got:abc"),
            "{seen:?}"
        );
        assert_eq!(reason, "FinalCommander has ended.");
    }
}
