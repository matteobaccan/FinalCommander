//! The filesystem watch behind `panel.auto_refresh` (SPEC 02 §1.25).
//!
//! A panel that was read ten minutes ago is showing ten minutes of history: a build that
//! wrote into the directory, a download that finished, a sibling program that moved a file
//! out. FAR notices; we did not, and `Ctrl+R` was the only answer.
//!
//! This is one thread, one `notify` watcher, and one rule: the directory a local panel is
//! looking at is watched, and nothing else. The thread sends [`Event::DirChanged`] on the
//! same channel the listings already arrive on, so the event loop needs no timer and the
//! program does no work at all while nothing changes.
//!
//! The debounce is the whole reason there is a thread rather than a callback. A `git
//! checkout` touches a thousand files in a few milliseconds; a thousand re-reads of the same
//! directory would be worse than not noticing. Within [`DEBOUNCE`] every change to a panel's
//! directory collapses into the single re-read it deserves.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use notify::{EventKind, RecursiveMode, Watcher as _};

use crate::app::Event;

/// How long the burst is allowed to go on before the panel is told. Long enough that an
/// unpack or a checkout is one re-read, short enough that a file that lands while you are
/// looking at the panel appears before you reach for `Ctrl+R`.
pub const DEBOUNCE: Duration = Duration::from_millis(300);

enum Msg {
    /// Watch `path` on behalf of `panel`; `None` stops watching for it.
    Set { panel: usize, path: Option<PathBuf> },
    /// Paths that changed, as the platform reported them.
    Touched(Vec<PathBuf>),
}

/// The handle the event loop keeps. Dropping it stops the thread.
pub struct Watch {
    tx: Sender<Msg>,
}

impl Watch {
    /// Start the watch. `events` is the channel the event loop reads.
    ///
    /// A platform that will not give us a watcher at all is not an error worth stopping the
    /// program for: the handle still works, it just never reports anything, and `Ctrl+R` is
    /// where it always was.
    pub fn start(events: Sender<Event>) -> Watch {
        let (tx, rx) = mpsc::channel::<Msg>();
        let feed = tx.clone();
        // This closure runs on `notify`'s own thread. All it does is forward paths: deciding
        // which panel they belong to, and when to say so, is the thread below.
        let watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res {
                if !matches!(event.kind, EventKind::Access(_)) && !event.paths.is_empty() {
                    let _ = feed.send(Msg::Touched(event.paths));
                }
            }
        });
        if let Ok(watcher) = watcher {
            let _ = std::thread::Builder::new()
                .name("watch".to_string())
                .spawn(move || run(watcher, rx, events));
        }
        Watch { tx }
    }

    /// What `panel` is looking at now, or `None` for a panel there is nothing to watch —
    /// a remote directory, the inside of an archive, a list of search results.
    pub fn set(&self, panel: usize, path: Option<PathBuf>) {
        let _ = self.tx.send(Msg::Set { panel, path });
    }
}

fn run(mut watcher: notify::RecommendedWatcher, rx: mpsc::Receiver<Msg>, events: Sender<Event>) {
    // Which directory each panel is on, and which directories we have told the platform
    // about. They are not the same thing: two panels on one directory are one watch.
    let mut dirs: HashMap<usize, PathBuf> = HashMap::new();
    let mut armed: HashSet<PathBuf> = HashSet::new();
    let mut pending: HashSet<usize> = HashSet::new();
    let mut since: Option<Instant> = None;
    loop {
        let msg = match since {
            None => rx.recv().ok(),
            Some(start) => match rx.recv_timeout(DEBOUNCE.saturating_sub(start.elapsed())) {
                Ok(msg) => Some(msg),
                Err(RecvTimeoutError::Timeout) => {
                    for panel in pending.drain() {
                        if events.send(Event::DirChanged { panel }).is_err() {
                            return;
                        }
                    }
                    since = None;
                    continue;
                }
                Err(RecvTimeoutError::Disconnected) => None,
            },
        };
        let Some(msg) = msg else { return };
        match msg {
            Msg::Set { panel, path } => {
                // The path the platform will report events under, which is not always the one
                // the panel shows: on macOS a directory under `/tmp` or `/var` is reached
                // through a symlink and FSEvents answers with `/private/...`, so a watch armed
                // on the name the user typed would never match anything that happened in it.
                // Resolved here, on this thread, because `canonicalize` touches the disk.
                match path {
                    Some(path) => {
                        let real = std::fs::canonicalize(&path).unwrap_or(path);
                        dirs.insert(panel, real)
                    }
                    None => dirs.remove(&panel),
                };
                let want: HashSet<PathBuf> = dirs.values().cloned().collect();
                for gone in armed.difference(&want) {
                    let _ = watcher.unwatch(gone);
                }
                for fresh in want.difference(&armed) {
                    // A directory that cannot be watched — a mount that went away, a handle
                    // limit — is simply not watched. The panel still works.
                    let _ = watcher.watch(fresh, RecursiveMode::NonRecursive);
                }
                armed = want;
                // A listing the panel has just asked for already has everything we were
                // about to tell it about.
                pending.remove(&panel);
                if pending.is_empty() {
                    since = None;
                }
            }
            Msg::Touched(paths) => {
                for (panel, dir) in &dirs {
                    if paths.iter().any(|p| p.starts_with(dir)) {
                        pending.insert(*panel);
                    }
                }
                if !pending.is_empty() && since.is_none() {
                    since = Some(Instant::now());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::RecvTimeoutError;

    /// Generous: a cold FSEvents stream on macOS takes its time, and a test that fails under
    /// load is worse than a test that waits.
    const WAIT: Duration = Duration::from_secs(10);

    fn next(rx: &mpsc::Receiver<Event>) -> Option<usize> {
        match rx.recv_timeout(WAIT) {
            Ok(Event::DirChanged { panel }) => Some(panel),
            Ok(_) => None,
            Err(_) => None,
        }
    }

    #[test]
    fn a_file_that_lands_in_the_watched_directory_tells_the_panel_once() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (tx, rx) = mpsc::channel::<Event>();
        let watch = Watch::start(tx);
        watch.set(0, Some(dir.path().to_path_buf()));
        // The watch is armed on the thread, not here: write until it is heard.
        let mut heard = None;
        for round in 0..20 {
            std::fs::write(dir.path().join(format!("f{round}")), b"x").expect("write");
            match rx.recv_timeout(Duration::from_millis(500)) {
                Ok(Event::DirChanged { panel }) => {
                    heard = Some(panel);
                    break;
                }
                Ok(_) => {}
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
        assert_eq!(
            heard,
            Some(0),
            "a file in the panel's directory is a change"
        );
        // The burst collapses: twenty files are not twenty events.
        for round in 0..20 {
            std::fs::write(dir.path().join(format!("g{round}")), b"x").expect("write");
        }
        assert_eq!(next(&rx), Some(0));
        assert!(
            rx.recv_timeout(DEBOUNCE * 4).is_err(),
            "one re-read per burst, not one per file"
        );
    }

    #[test]
    fn a_directory_nobody_is_looking_at_is_not_watched() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (tx, rx) = mpsc::channel::<Event>();
        let watch = Watch::start(tx);
        watch.set(0, Some(dir.path().to_path_buf()));
        watch.set(0, None);
        // Give the thread time to drop the watch before anything happens in there.
        std::thread::sleep(DEBOUNCE * 2);
        std::fs::write(dir.path().join("late"), b"x").expect("write");
        assert!(
            rx.recv_timeout(DEBOUNCE * 4).is_err(),
            "a panel that left says nothing"
        );
    }
}
