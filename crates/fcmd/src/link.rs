//! The Link on the runtime (SPEC 04 §9): the dialer the netbox calls with, and the called
//! side — the listener, the answer to broadcasts, and the bridge between a call and the
//! dialog that asks the person here about it.
//!
//! The loop owns one [`Hub`]. A call arrives on the runtime, becomes [`Event::LinkCall`],
//! and waits on a one-shot channel for [`Hub::answer`], which the loop calls when the dialog
//! is answered. A call nobody answers is dropped by the server after a minute; the guard
//! then tells the loop to take the dialog down.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};

use async_trait::async_trait;
use fcmd_net::link::pin::Pin;
use fcmd_net::link::{self, discovery, Admission, Caller, Gate, HangUp, Identity, Share};
use tokio::sync::{oneshot, watch};

use crate::app::{Event, LinkCall};

type Pending = Arc<Mutex<HashMap<u64, oneshot::Sender<bool>>>>;

/// Where calls come in. `link.port` when it is set; otherwise 47831, the port a caller
/// that names this machine by its IP address tries — and, when another program holds that
/// one, any port, which the broadcast tells callers on this network.
async fn listen(port: u16) -> std::io::Result<tokio::net::TcpListener> {
    if port != 0 {
        return tokio::net::TcpListener::bind(("0.0.0.0", port)).await;
    }
    match tokio::net::TcpListener::bind(("0.0.0.0", discovery::PORT)).await {
        Ok(listener) => Ok(listener),
        Err(_) => tokio::net::TcpListener::bind(("0.0.0.0", 0)).await,
    }
}

/// The ID, and the address a caller on another network types instead (`192.168.1.10`, with
/// `:port` when it is not the usual one).
fn reachable_as(id: u32, port: u16) -> String {
    let id = link::format_id(id);
    match discovery::default_address() {
        Some(ip) if port == discovery::PORT => format!("{id} · {ip}"),
        Some(ip) => format!("{id} · {ip}:{port}"),
        None => id,
    }
}

/// The name callers see when the settings give none: the machine's own.
pub fn machine_name() -> String {
    let from_env = ["COMPUTERNAME", "HOSTNAME"]
        .iter()
        .filter_map(|v| std::env::var(v).ok())
        .find(|n| !n.trim().is_empty());
    let name = from_env.or_else(|| {
        std::fs::read_to_string("/etc/hostname")
            .ok()
            .map(|n| n.trim().to_string())
            .filter(|n| !n.is_empty())
    });
    name.unwrap_or_else(|| "FinalCommander".to_string())
}

pub struct Hub {
    dir: PathBuf,
    name: String,
    tx: mpsc::Sender<Event>,
    pending: Pending,
    next: Arc<AtomicU64>,
    hang_up: watch::Sender<HangUp>,
    running: Option<tokio::task::JoinHandle<()>>,
    /// `link.port`: 0 for one the system chooses.
    pub port: u16,
}

impl Hub {
    /// `dir` holds the identity and the fingerprints met before; `name` is what the other
    /// screen shows.
    pub fn new(dir: PathBuf, name: String, tx: mpsc::Sender<Event>) -> Hub {
        Hub {
            dir,
            name,
            tx,
            pending: Arc::default(),
            next: Arc::new(AtomicU64::new(1)),
            hang_up: watch::channel(HangUp::default()).0,
            running: None,
            port: 0,
        }
    }

    /// What the pool calls `link://` sites with.
    pub fn dialer(&self) -> link::Dialer {
        let tx = self.tx.clone();
        link::Dialer {
            dir: self.dir.clone(),
            name: self.name.clone(),
            known: self.dir.join("link-known"),
            show_code: Arc::new(move |site, code| {
                let _ = tx.send(Event::LinkCode {
                    host: site.host.clone(),
                    code: code.to_string(),
                });
            }),
        }
    }

    /// Start accepting calls with `share` offered read only, or stop (`None`).
    pub fn accept(&mut self, runtime: &tokio::runtime::Handle, share: Option<PathBuf>) {
        if let Some(task) = self.running.take() {
            task.abort();
            self.hang_up();
            self.pending.lock().expect("link").clear();
            if share.is_none() {
                let _ = self.tx.send(Event::LinkAccepting { result: Ok(None) });
            }
        }
        let Some(share) = share else {
            return;
        };
        let label = share
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| share.display().to_string());
        let offered = match Share::new(&label, &share, false) {
            Ok(offered) => offered,
            Err(e) => {
                let _ = self.tx.send(Event::LinkAccepting {
                    result: Err(format!("{}: {e}", share.display())),
                });
                return;
            }
        };
        let gate = Arc::new(Bridge {
            tx: self.tx.clone(),
            pending: self.pending.clone(),
            next: self.next.clone(),
            share: Some(offered),
            hang_up: self.hang_up.subscribe(),
            dir: self.dir.clone(),
            told: AtomicU64::new(0),
            began: std::time::Instant::now(),
        });
        let (dir, name, tx, port) = (
            self.dir.clone(),
            self.name.clone(),
            self.tx.clone(),
            self.port,
        );
        self.running = Some(runtime.spawn(async move {
            if let Err(e) = run(&dir, &name, &share, port, gate, &tx).await {
                let _ = tx.send(Event::LinkAccepting {
                    result: Err(e.to_string()),
                });
            }
        }));
    }

    /// The dialog's answer to call `call`.
    pub fn answer(&self, call: u64, accept: bool) {
        if let Some(reply) = self.pending.lock().expect("link").remove(&call) {
            let _ = reply.send(accept);
        }
    }

    /// End every session open here.
    pub fn hang_up(&self) {
        self.hang_up.send_modify(|h| {
            h.generation += 1;
            h.only = None;
        });
    }
}

async fn run(
    dir: &Path,
    name: &str,
    folder: &Path,
    port: u16,
    gate: Arc<Bridge>,
    tx: &mpsc::Sender<Event>,
) -> std::io::Result<()> {
    let identity = Arc::new(Identity::load_or_create(dir, name)?);
    let listener = listen(port).await?;
    let port = listener.local_addr()?.port();
    let udp = tokio::net::UdpSocket::bind(("0.0.0.0", discovery::PORT))
        .await
        .map_err(|e| {
            std::io::Error::other(format!(
                "UDP port {} for being found: {e} — is another FinalCommander here accepting \
                 calls?",
                discovery::PORT
            ))
        })?;
    let _ = tx.send(Event::LinkAccepting {
        result: Ok(Some((
            reachable_as(identity.id, port),
            folder.display().to_string(),
        ))),
    });
    let found = discovery::answer(udp, identity.id, identity.name.clone(), port);
    let served = link::serve(listener, identity, gate);
    tokio::select! {
        result = found => result,
        result = served => result,
    }
}

/// The server's side of the dialog.
struct Bridge {
    tx: mpsc::Sender<Event>,
    pending: Pending,
    next: Arc<AtomicU64>,
    share: Option<Share>,
    hang_up: watch::Receiver<HangUp>,
    /// Where the PIN is kept, read at each call so that `fcmd --link-pin` counts at once.
    dir: PathBuf,
    /// When the loop was last told of traffic, in milliseconds since the bridge began: told
    /// at most every 150 ms, which is plenty for a light and nothing for the loop.
    told: AtomicU64,
    began: std::time::Instant,
}

/// Takes the dialog down when the server stops waiting for it.
struct Unanswered<'a> {
    call: u64,
    bridge: &'a Bridge,
    done: bool,
}

impl Drop for Unanswered<'_> {
    fn drop(&mut self) {
        if !self.done {
            self.bridge.pending.lock().expect("link").remove(&self.call);
            let _ = self.bridge.tx.send(Event::LinkCallGone { call: self.call });
        }
    }
}

#[async_trait]
impl Gate for Bridge {
    fn shares(&self) -> Vec<Share> {
        self.share.iter().cloned().collect()
    }

    async fn admit(&self, caller: &Caller) -> Admission {
        let call = self.next.fetch_add(1, Ordering::Relaxed);
        let (reply, answer) = oneshot::channel();
        self.pending.lock().expect("link").insert(call, reply);
        let mut guard = Unanswered {
            call,
            bridge: self,
            done: false,
        };
        let _ = self.tx.send(Event::LinkCall(LinkCall {
            call,
            id: link::format_id(caller.id),
            name: caller.name.clone(),
            code: caller.sas.clone(),
            addr: caller.addr.ip().to_string(),
            share: caller.share.clone(),
        }));
        let accepted = answer.await.unwrap_or(false);
        guard.done = true;
        if accepted {
            Admission::ReadOnly
        } else {
            Admission::Refused
        }
    }

    fn opened(&self, caller: &Caller, _write: bool) {
        let _ = self.tx.send(Event::LinkSession {
            name: caller.name.clone(),
            share: caller.share.clone(),
            open: true,
        });
    }

    fn closed(&self, caller: &Caller) {
        let _ = self.tx.send(Event::LinkSession {
            name: caller.name.clone(),
            share: caller.share.clone(),
            open: false,
        });
    }

    fn traffic(&self) {
        let now = self.began.elapsed().as_millis() as u64 + 1;
        let last = self.told.load(Ordering::Relaxed);
        if now.saturating_sub(last) >= 150
            && self
                .told
                .compare_exchange(last, now, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
        {
            let _ = self.tx.send(Event::LinkTraffic);
        }
    }

    fn hang_up(&self) -> Option<watch::Receiver<HangUp>> {
        Some(self.hang_up.clone())
    }

    fn pin(&self) -> Option<Pin> {
        Pin::load(&self.dir).ok().flatten()
    }
}

/// `fcmd --link-pin`: set the PIN, or take it away with an empty one.
pub fn set_pin(dir: &Path) -> anyhow::Result<()> {
    let first = fcmd_tui::read_hidden_line("New Link PIN (empty removes it): ")?;
    if first.trim().is_empty() {
        Pin::remove(dir)?;
        println!("The Link PIN is removed: calls go to the person at this machine again.");
        return Ok(());
    }
    let pin = Pin::new(&first)?;
    let again = fcmd_tui::read_hidden_line("The same PIN again: ")?;
    if again.trim() != first.trim() {
        anyhow::bail!("the two PINs differ: nothing was changed");
    }
    pin.save(dir)?;
    println!(
        "The Link PIN is set ({}). A caller gives it as the netbox session's password.",
        dir.join(link::pin::PIN_FILE).display()
    );
    Ok(())
}

/// `fcmd --link-serve`: accept calls with no screen, until the process is stopped. Only a
/// caller with the PIN gets in — there is nobody to ask — so there has to be one.
pub fn serve_headless(dir: &Path, name: &str, folder: &Path, port: u16) -> anyhow::Result<()> {
    if Pin::load(dir)?.is_none() {
        anyhow::bail!(
            "a Link that answers by itself lets in only callers with its PIN, and none is \
             set: run `fcmd --link-pin` first"
        );
    }
    let label = folder
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| folder.display().to_string());
    let share = Share::new(&label, folder, false)
        .map_err(|e| anyhow::anyhow!("{}: {e}", folder.display()))?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;
    runtime.block_on(async move {
        let identity = Arc::new(Identity::load_or_create(dir, name)?);
        let listener = listen(port)
            .await
            .map_err(|e| anyhow::anyhow!("TCP port {port}: {e}"))?;
        let port = listener.local_addr()?.port();
        let udp = tokio::net::UdpSocket::bind(("0.0.0.0", discovery::PORT))
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "UDP port {} for being found: {e} — is another FinalCommander here \
                     accepting calls?",
                    discovery::PORT
                )
            })?;
        println!(
            "FinalCommander Link: ID {} ({}), TCP port {port}, sharing {} read only; callers \
             need the PIN.",
            reachable_as(identity.id, port),
            identity.name,
            folder.display()
        );
        let gate = Arc::new(Unattended {
            dir: dir.to_path_buf(),
            share,
        });
        let found = discovery::answer(udp, identity.id, identity.name.clone(), port);
        let served = link::serve(listener, identity, gate);
        tokio::select! {
            result = found => result?,
            result = served => result?,
        }
        Ok(())
    })
}

/// The gate of a Link nobody sits at: the PIN or nothing, and a line for each call.
struct Unattended {
    dir: PathBuf,
    share: Share,
}

fn log(what: &str, caller: &Caller) {
    println!(
        "{} {what}: {} (ID {}, {})",
        fcmd_ui::format_stamp(std::time::SystemTime::now()),
        caller.name,
        link::format_id(caller.id),
        caller.addr.ip()
    );
}

#[async_trait]
impl Gate for Unattended {
    fn shares(&self) -> Vec<Share> {
        vec![self.share.clone()]
    }

    async fn admit(&self, _caller: &Caller) -> Admission {
        Admission::Refused
    }

    fn attended(&self) -> bool {
        false
    }

    fn pin(&self) -> Option<Pin> {
        Pin::load(&self.dir).ok().flatten()
    }

    fn opened(&self, caller: &Caller, _write: bool) {
        log("in, by its PIN", caller);
    }

    fn closed(&self, caller: &Caller) {
        log("left", caller);
    }

    fn refused(&self, caller: &Caller, why: &str) {
        log(&format!("refused, {why}"), caller);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A call through the hub: the dialog's event arrives, the answer lets the caller in, and
    /// a hang-up from this side ends the session.
    #[test]
    fn a_call_reaches_the_dialog_and_its_answer_reaches_the_caller() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        let here = tempfile::tempdir().unwrap();
        let shared = tempfile::tempdir().unwrap();
        std::fs::write(shared.path().join("note.txt"), b"hello").unwrap();
        let (tx, rx) = mpsc::channel();
        let hub = Hub::new(here.path().to_path_buf(), "here".into(), tx.clone());
        let gate = Arc::new(Bridge {
            tx: tx.clone(),
            pending: hub.pending.clone(),
            next: hub.next.clone(),
            share: Some(Share::new("shared", shared.path(), false).unwrap()),
            hang_up: hub.hang_up.subscribe(),
            dir: here.path().to_path_buf(),
            told: AtomicU64::new(0),
            began: std::time::Instant::now(),
        });
        let identity = Arc::new(Identity::load_or_create(here.path(), "here").unwrap());
        let listener = runtime
            .block_on(tokio::net::TcpListener::bind(("127.0.0.1", 0)))
            .unwrap();
        let addr = listener.local_addr().unwrap();
        runtime.spawn(link::serve(listener, identity, gate));

        let caller_dir = tempfile::tempdir().unwrap();
        let caller = Identity::load_or_create(caller_dir.path(), "there").unwrap();
        let pending = runtime.block_on(link::call(addr, &caller)).unwrap();
        let asked = runtime.spawn(async move { pending.ask("", false).await });
        let call = match rx.recv_timeout(std::time::Duration::from_secs(10)).unwrap() {
            Event::LinkCall(call) => call,
            other => panic!("a call was expected, not {other:?}"),
        };
        assert_eq!(call.name, "there");
        assert_eq!(call.share, "shared");
        hub.answer(call.call, true);
        let fs = runtime.block_on(asked).unwrap().unwrap();
        let rows = runtime
            .block_on(fcmd_net::RemoteFs::read_dir(&fs, "/"))
            .unwrap();
        assert!(rows.iter().any(|r| r.name == "note.txt"));
        loop {
            match rx.recv_timeout(std::time::Duration::from_secs(10)).unwrap() {
                Event::LinkSession { open: true, .. } => break,
                Event::LinkTraffic => continue,
                other => panic!("the session was expected to open, not {other:?}"),
            }
        }
        hub.hang_up();
        loop {
            match rx.recv_timeout(std::time::Duration::from_secs(10)).unwrap() {
                Event::LinkSession { open: false, .. } => break,
                Event::LinkTraffic => continue,
                other => panic!("the session was expected to close, not {other:?}"),
            }
        }
    }

    /// A machine on another network is called by its address: no broadcast, straight to the
    /// port, the dialog there as usual, and the folder in the panel.
    #[test]
    fn a_machine_named_by_its_address_is_called_without_a_broadcast() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        let here = tempfile::tempdir().unwrap();
        let shared = tempfile::tempdir().unwrap();
        std::fs::write(shared.path().join("note.txt"), b"hello").unwrap();
        let (tx, rx) = mpsc::channel();
        let hub = Hub::new(here.path().to_path_buf(), "here".into(), tx.clone());
        let gate = Arc::new(Bridge {
            tx: tx.clone(),
            pending: hub.pending.clone(),
            next: hub.next.clone(),
            share: Some(Share::new("shared", shared.path(), false).unwrap()),
            hang_up: hub.hang_up.subscribe(),
            dir: here.path().to_path_buf(),
            told: AtomicU64::new(0),
            began: std::time::Instant::now(),
        });
        let identity = Arc::new(Identity::load_or_create(here.path(), "here").unwrap());
        let listener = runtime
            .block_on(tokio::net::TcpListener::bind(("127.0.0.1", 0)))
            .unwrap();
        let port = listener.local_addr().unwrap().port();
        runtime.spawn(link::serve(listener, identity, gate));

        let there = tempfile::tempdir().unwrap();
        let (tx2, rx2) = mpsc::channel();
        let pool = fcmd_net::Pool::new();
        pool.set_link(Hub::new(there.path().to_path_buf(), "there".into(), tx2).dialer());
        let site = fcmd_net::Site::from_config(&fcmd_config::SiteCfg {
            id: "office".into(),
            scheme: "link".into(),
            host: "127.0.0.1".into(),
            port: Some(port),
            ..Default::default()
        })
        .unwrap();
        let start = site.start();
        let opened = runtime.spawn(async move {
            let remote = pool.connect(&site, None).await.map_err(|e| e.to_string())?;
            let rows = remote
                .sftp()
                .read_dir("/")
                .await
                .map_err(|e| e.to_string())?;
            Ok::<_, String>(rows.into_iter().map(|e| e.name).collect::<Vec<_>>())
        });
        let call = match rx.recv_timeout(std::time::Duration::from_secs(10)).unwrap() {
            Event::LinkCall(call) => call,
            other => panic!("a call was expected, not {other:?}"),
        };
        match rx2
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap()
        {
            Event::LinkCode { code, .. } => assert_eq!(code, call.code),
            other => panic!("the code was expected, not {other:?}"),
        }
        hub.answer(call.call, true);
        let names = runtime.block_on(opened).unwrap().unwrap();
        assert_eq!(names, ["note.txt"]);
        assert!(start.to_string().contains("127.0.0.1"));
    }
}
