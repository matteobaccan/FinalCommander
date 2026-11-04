use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Cooperative cancellation, shared by directory scans, previews and jobs.
///
/// Leaving a directory cancels its scan; `Esc` on a running job cancels its units. Every loop
/// that can run long checks this.
///
/// It also pauses. A paused token makes [`is_cancelled`](Self::is_cancelled) wait until it is
/// resumed or cancelled, so every place an engine already asks "should I stop?" is a place it
/// can be held, without one line of the engines knowing about pauses (SPEC 03 §5). The thread
/// asking is the worker's, never the screen's: only the queue pauses a token, and only the
/// token of a job.
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<Flags>);

#[derive(Debug, Default)]
struct Flags {
    cancelled: AtomicBool,
    paused: AtomicBool,
}

/// Two tokens are equal when they are the *same* token, not when their flags happen to match:
/// a job and its cancel button share one, and that is the question callers ask.
impl PartialEq for CancelToken {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for CancelToken {}

impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.cancelled.store(true, Ordering::Relaxed);
    }

    /// Has somebody asked to stop? While the token is paused this waits, and answers when it
    /// is resumed (no) or cancelled (yes).
    pub fn is_cancelled(&self) -> bool {
        while self.0.paused.load(Ordering::Relaxed) && !self.0.cancelled.load(Ordering::Relaxed) {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        self.0.cancelled.load(Ordering::Relaxed)
    }

    /// Hold the work at the next place it checks this token.
    pub fn pause(&self) {
        self.0.paused.store(true, Ordering::Relaxed);
    }

    pub fn resume(&self) {
        self.0.paused.store(false, Ordering::Relaxed);
    }

    pub fn is_paused(&self) -> bool {
        self.0.paused.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equality_is_identity_not_state() {
        let a = CancelToken::new();
        let b = a.clone();
        let other = CancelToken::new();
        assert_eq!(a, b);
        assert_ne!(a, other);
        other.cancel();
        assert_ne!(a, other);
    }

    #[test]
    fn a_clone_shares_the_flag() {
        let a = CancelToken::new();
        let b = a.clone();
        assert!(!b.is_cancelled());
        a.cancel();
        assert!(b.is_cancelled());
    }

    #[test]
    fn a_paused_token_holds_the_worker_until_it_is_resumed() {
        let token = CancelToken::new();
        token.pause();
        let worker = {
            let token = token.clone();
            std::thread::spawn(move || {
                let started = std::time::Instant::now();
                let stop = token.is_cancelled();
                (stop, started.elapsed())
            })
        };
        std::thread::sleep(std::time::Duration::from_millis(200));
        assert!(token.is_paused());
        token.resume();
        let (stop, waited) = worker.join().unwrap();
        assert!(!stop, "a resumed job goes on");
        assert!(waited >= std::time::Duration::from_millis(150));
    }

    #[test]
    fn cancelling_a_paused_token_lets_the_worker_stop() {
        let token = CancelToken::new();
        token.pause();
        let worker = {
            let token = token.clone();
            std::thread::spawn(move || token.is_cancelled())
        };
        token.cancel();
        assert!(worker.join().unwrap());
    }
}
