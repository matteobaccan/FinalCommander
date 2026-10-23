use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Cooperative cancellation, shared by directory scans, previews and jobs.
///
/// Leaving a directory cancels its scan; `Esc` on a running job cancels its units. Every loop
/// that can run long checks this.
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

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
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
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
}
