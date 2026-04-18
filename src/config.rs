use std::net::SocketAddr;

use serde::Deserialize;

/// Jitter applied before injecting the fake ClientHello packet.
/// Randomizing the delay defeats timing-based DPI fingerprinting
/// (a fixed 1 ms delay is detectable; a random 1–8 ms range is not).
/// Enabled by default — set max_ms = 0 to disable.
#[derive(Debug, Deserialize, Clone)]
pub struct JitterConfig {
    /// Minimum delay in milliseconds. Default: 1.
    #[serde(default = "JitterConfig::default_min")]
    pub min_ms: u64,
    /// Maximum delay in milliseconds. Default: 8. Set to 0 to disable jitter entirely.
    #[serde(default = "JitterConfig::default_max")]
    pub max_ms: u64,
}

impl JitterConfig {
    fn default_min() -> u64 { 1 }
    fn default_max() -> u64 { 8 }

    pub fn is_disabled(&self) -> bool {
        self.max_ms == 0
    }
}

impl Default for JitterConfig {
    fn default() -> Self {
        Self { min_ms: 1, max_ms: 8 }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct TimeoutConfig {
    /// TCP handshake timeout in milliseconds. Default: 5000 (5 seconds).
    /// Increase if connecting to slow/distant servers; decrease for faster failure detection.
    #[serde(default = "TimeoutConfig::default_handshake")]
    pub handshake_timeout_ms: u64,
    /// Sniffer confirmation timeout in milliseconds. Default: 2000 (2 seconds).
    /// Time to wait for the fake packet injection to be confirmed.
    #[serde(default = "TimeoutConfig::default_confirmation")]
    pub confirmation_timeout_ms: u64,
}

impl TimeoutConfig {
    fn default_handshake() -> u64 { 5000 }
    fn default_confirmation() -> u64 { 2000 }
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            handshake_timeout_ms: 5000,
            confirmation_timeout_ms: 2000,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub listeners: Vec<ListenerConfig>,
    /// Suppress repeated identical log messages within a time window.
    /// Useful in production to avoid log flooding. Default: false.
    #[serde(default)]
    pub debounce_logs: bool,
    /// Randomized delay before fake packet injection to defeat timing-based DPI.
    /// Default: enabled (1–8 ms). Set max_ms to 0 to disable.
    #[serde(default)]
    pub jitter: JitterConfig,
    /// Connection timeout settings.
    #[serde(default)]
    pub timeouts: TimeoutConfig,
}

#[derive(Debug, Deserialize)]
pub struct ListenerConfig {
    pub listen: SocketAddr,
    pub connect: SocketAddr,
    pub fake_sni: String,
    /// Gaming mode: smaller socket buffers for lower latency at the cost of throughput.
    /// Default: false (high-throughput mode).
    #[serde(default)]
    pub gaming_mode: bool,
}

pub fn load(path: &str) -> Result<Config, crate::error::ConfigError> {
    let data = std::fs::read_to_string(path)
        .map_err(|e| crate::error::ConfigError::Io(path.to_string(), e))?;
    let cfg: Config = serde_json::from_str(&data)
        .map_err(|e| crate::error::ConfigError::Parse(path.to_string(), e))?;
    if cfg.listeners.is_empty() {
        return Err(crate::error::ConfigError::Empty);
    }
    for lc in &cfg.listeners {
        if lc.fake_sni.len() > 219 {
            return Err(crate::error::ConfigError::SniTooLong(lc.fake_sni.clone()));
        }
    }
    Ok(cfg)
}
