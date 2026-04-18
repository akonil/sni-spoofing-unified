use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

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

/// Per-SNI success/failure counters.
pub struct SniEntry {
    pub success: AtomicU64,
    pub failure: AtomicU64,
}

/// Tracks how often each fake SNI leads to a successful DPI bypass vs. failure.
/// Shared across all handlers via Arc.
pub struct SniTracker {
    entries: Mutex<HashMap<String, Arc<SniEntry>>>,
}

impl SniTracker {
    pub fn new() -> Arc<Self> {
        Arc::new(Self { entries: Mutex::new(HashMap::new()) })
    }

    fn get_or_create(&self, sni: &str) -> Arc<SniEntry> {
        let mut map = self.entries.lock().unwrap();
        map.entry(sni.to_string())
            .or_insert_with(|| Arc::new(SniEntry {
                success: AtomicU64::new(0),
                failure: AtomicU64::new(0),
            }))
            .clone()
    }

    pub fn record_success(&self, sni: &str) {
        self.get_or_create(sni).success.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_failure(&self, sni: &str) {
        self.get_or_create(sni).failure.fetch_add(1, Ordering::Relaxed);
    }

    /// Returns a snapshot sorted by success count descending.
    pub fn snapshot(&self) -> Vec<(String, u64, u64)> {
        let map = self.entries.lock().unwrap();
        let mut v: Vec<_> = map.iter()
            .map(|(k, e)| (
                k.clone(),
                e.success.load(Ordering::Relaxed),
                e.failure.load(Ordering::Relaxed),
            ))
            .collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v
    }
}
