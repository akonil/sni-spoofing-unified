use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;

/// Per-listener connection counters, cheaply shared across tasks via Arc.
pub struct Stats {
    /// Connections currently open and relaying data.
    active: AtomicI64,
    /// Total connections handled since startup (including closed ones).
    total: AtomicU64,
}

impl Stats {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            active: AtomicI64::new(0),
            total: AtomicU64::new(0),
        })
    }

    /// Call when a new connection is accepted.
    pub fn opened(&self) {
        self.active.fetch_add(1, Ordering::Relaxed);
        self.total.fetch_add(1, Ordering::Relaxed);
    }

    /// Call when a connection closes (success or error).
    pub fn closed(&self) {
        self.active.fetch_sub(1, Ordering::Relaxed);
    }

    /// Return (active, total) snapshot.
    pub fn snapshot(&self) -> (i64, u64) {
        (
            self.active.load(Ordering::Relaxed),
            self.total.load(Ordering::Relaxed),
        )
    }
}

/// RAII guard — increments on creation, decrements on drop.
/// Wrap it in a local variable so the counter is always released
/// even when the handler returns early or panics.
pub struct ConnectionGuard(Arc<Stats>);

impl ConnectionGuard {
    pub fn new(stats: Arc<Stats>) -> Self {
        stats.opened();
        ConnectionGuard(stats)
    }

    pub fn snapshot(&self) -> (i64, u64) {
        self.0.snapshot()
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.0.closed();
    }
}
