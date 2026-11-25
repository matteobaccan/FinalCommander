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
use fcmd_net::link::{self, discovery, Admission, Caller, Gate, Identity, Share};
use tokio::sync::{oneshot, watch};

use crate::app::{Event, LinkCall};

type Pending = Arc<Mutex<HashMap<u64, oneshot::Sender<bool>>>>;

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
    hang_up: watch::Sender<u64>,
    running: Option<tokio::task::JoinHandle<()>>,
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
            hang_up: watch::channel(0).0,
            running: None,
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
        });
        let (dir, name, tx) = (self.dir.clone(), self.name.clone(), self.tx.clone());
        self.running = Some(runtime.spawn(async move {
            if let Err(e) = run(&dir, &name, &share, gate, &tx).await {
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
        self.hang_up.send_modify(|n| *n += 1);
    }
}

async fn run(
    dir: &Path,
    name: &str,
    folder: &Path,
    gate: Arc<Bridge>,
    tx: &mpsc::Sender<Event>,
) -> std::io::Result<()> {
    let identity = Arc::new(Identity::load_or_create(dir, name)?);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", 0)).await?;
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
            link::format_id(identity.id),
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
    hang_up: watch::Receiver<u64>,
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

    fn hang_up(&self) -> Option<watch::Receiver<u64>> {
        Some(self.hang_up.clone())
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
        match rx.recv_timeout(std::time::Duration::from_secs(10)).unwrap() {
            Event::LinkSession { open: true, .. } => {}
            other => panic!("the session was expected to open, not {other:?}"),
        }
        hub.hang_up();
        match rx.recv_timeout(std::time::Duration::from_secs(10)).unwrap() {
            Event::LinkSession { open: false, .. } => {}
            other => panic!("the session was expected to close, not {other:?}"),
        }
    }
}
