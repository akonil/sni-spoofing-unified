use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Default debounce interval for log messages
pub const DEFAULT_DEBOUNCE_SECS: u64 = 5;

/// Global toggle — set once at startup from Config::debounce_logs.
static DEBOUNCE_ENABLED: AtomicBool = AtomicBool::new(false);

/// Call once at startup to enable debounced logging.
pub fn set_enabled(enabled: bool) {
    DEBOUNCE_ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn is_enabled() -> bool {
    DEBOUNCE_ENABLED.load(Ordering::Relaxed)
}

/// Tracks suppressed message counts per message key
static SUPPRESSED_COUNTS: Mutex<Option<HashMap<String, u64>>> = Mutex::new(None);

/// Gets current timestamp in seconds
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Debounce configuration for a specific message type
pub struct DebounceConfig {
    /// Unique key to identify this message type
    pub key: &'static str,
    /// Debounce interval in seconds
    pub interval_secs: u64,
}

impl DebounceConfig {
    /// Create a new debounce config with the given key and default 5-second interval
    pub const fn new(key: &'static str) -> Self {
        Self {
            key,
            interval_secs: DEFAULT_DEBOUNCE_SECS,
        }
    }

    /// Set a custom debounce interval
    pub const fn with_interval(mut self, secs: u64) -> Self {
        self.interval_secs = secs;
        self
    }
}

/// State stored for each debounced message type
struct DebounceState {
    last_logged_secs: AtomicU64,
}

impl DebounceState {
    fn new() -> Self {
        Self {
            last_logged_secs: AtomicU64::new(0),
        }
    }
}

/// Global registry of debounce states
static DEBOUNCE_STATES: Mutex<Option<HashMap<&'static str, DebounceState>>> = Mutex::new(None);

/// Check if a message should be logged (not suppressed)
/// Returns (should_log, suppressed_count)
pub fn should_log(key: &'static str, interval_secs: u64) -> (bool, u64) {
    let now = now_secs();

    let mut states_guard = DEBOUNCE_STATES.lock().unwrap();
    if states_guard.is_none() {
        *states_guard = Some(HashMap::new());
    }
    let states = states_guard.as_mut().unwrap();

    let state = states.entry(key).or_insert_with(DebounceState::new);
    let last = state.last_logged_secs.load(Ordering::Relaxed);

    if now - last >= interval_secs {
        // Time to log - reset and return suppressed count
        state.last_logged_secs.store(now, Ordering::Relaxed);

        // Get and reset suppressed count
        let mut counts_guard = SUPPRESSED_COUNTS.lock().unwrap();
        if counts_guard.is_none() {
            *counts_guard = Some(HashMap::new());
        }
        let counts = counts_guard.as_mut().unwrap();
        let suppressed = counts.remove(key).unwrap_or(0);

        (true, suppressed)
    } else {
        // Suppress this message
        drop(states_guard);

        let mut counts_guard = SUPPRESSED_COUNTS.lock().unwrap();
        if counts_guard.is_none() {
            *counts_guard = Some(HashMap::new());
        }
        let counts = counts_guard.as_mut().unwrap();
        *counts.entry(key.to_string()).or_insert(0) += 1;

        (false, 0)
    }
}

/// Debounced log macro. When debounce_logs is disabled in config, logs every message immediately.
/// Usage: log_debounced!("handler_error", warn, "message", field1 = value1)
#[macro_export]
macro_rules! log_debounced {
    ($key:expr, $level:ident, $($arg:tt)*) => {
        {
            if !$crate::debounce::is_enabled() {
                tracing::$level!($($arg)*);
            } else {
                let (should_log, suppressed) = $crate::debounce::should_log($key, $crate::debounce::DEFAULT_DEBOUNCE_SECS);
                if should_log {
                    if suppressed > 0 {
                        tracing::$level!(suppressed = suppressed, $($arg)*);
                    } else {
                        tracing::$level!($($arg)*);
                    }
                }
            }
        }
    };
}

/// Log with a custom debounce interval. Falls through immediately when debouncing is disabled.
#[macro_export]
macro_rules! log_debounced_interval {
    ($key:expr, $interval_secs:expr, $level:ident, $($arg:tt)*) => {
        {
            if !$crate::debounce::is_enabled() {
                tracing::$level!($($arg)*);
            } else {
                let (should_log, suppressed) = $crate::debounce::should_log($key, $interval_secs);
                if should_log {
                    if suppressed > 0 {
                        tracing::$level!(suppressed = suppressed, $($arg)*);
                    } else {
                        tracing::$level!($($arg)*);
                    }
                }
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debounce_basic() {
        // First call should log
        let (ok1, sup1) = should_log("test_key", 5);
        assert!(ok1);
        assert_eq!(sup1, 0);

        // Immediate second call should not log
        let (ok2, sup2) = should_log("test_key", 5);
        assert!(!ok2);
        assert_eq!(sup2, 0);

        // Different key should log
        let (ok3, sup3) = should_log("test_key_2", 5);
        assert!(ok3);
        assert_eq!(sup3, 0);
    }
}
