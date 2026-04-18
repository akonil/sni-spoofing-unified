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

/// Adds random extra bytes to the fake ClientHello to vary its size fingerprint.
/// Disabled by default (max_extra_bytes = 0).
#[derive(Debug, Deserialize, Clone)]
pub struct PayloadPaddingConfig {
    /// Minimum extra bytes to add. Default: 0.
    #[serde(default)]
    pub min_extra_bytes: usize,
    /// Maximum extra bytes to add. Default: 0 (disabled). Set > 0 to enable.
    #[serde(default)]
    pub max_extra_bytes: usize,
}

impl Default for PayloadPaddingConfig {
    fn default() -> Self { Self { min_extra_bytes: 0, max_extra_bytes: 0 } }
}

impl PayloadPaddingConfig {
    pub fn is_disabled(&self) -> bool { self.max_extra_bytes == 0 }
}

/// Splits the fake ClientHello into multiple TCP segments to confuse DPI reassembly.
/// Disabled by default.
#[derive(Debug, Deserialize, Clone)]
pub struct FragmentationConfig {
    /// Enable fragmentation. Default: false.
    #[serde(default)]
    pub enabled: bool,
    /// Number of fragments to split the fake payload into (2 or 3). Default: 2.
    #[serde(default = "FragmentationConfig::default_fragments")]
    pub fragments: usize,
    /// Millisecond delay between fragments. Default: 1.
    #[serde(default = "FragmentationConfig::default_delay_ms")]
    pub delay_ms: u64,
}

impl FragmentationConfig {
    fn default_fragments() -> usize { 2 }
    fn default_delay_ms() -> u64 { 1 }
}

impl Default for FragmentationConfig {
    fn default() -> Self { Self { enabled: false, fragments: 2, delay_ms: 1 } }
}

/// Advanced DPI-evasion options. All features are disabled by default.
#[derive(Debug, Deserialize, Clone, Default)]
pub struct AdvancedConfig {
    /// Vary the fake ClientHello size to avoid fixed-length fingerprinting.
    #[serde(default)]
    pub payload_padding: PayloadPaddingConfig,
    /// Split the fake ClientHello into multiple TCP segments.
    #[serde(default)]
    pub fragmentation: FragmentationConfig,
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
    /// Advanced DPI-evasion features (payload padding, fragmentation).
    #[serde(default)]
    pub advanced: AdvancedConfig,
}

#[derive(Debug, Deserialize)]
pub struct ListenerConfig {
    pub listen: SocketAddr,
    pub connect: SocketAddr,
    /// Single fake SNI (legacy). Use fake_sni_pool for rotation across multiple SNIs.
    #[serde(default)]
    pub fake_sni: Option<String>,
    /// Pool of fake SNIs — one is chosen at random per connection.
    /// If set, takes precedence over fake_sni.
    #[serde(default)]
    pub fake_sni_pool: Vec<String>,
    /// Max accepted connections per second for this listener. 0 = unlimited. Default: 0.
    #[serde(default)]
    pub max_connections_per_sec: u32,
    /// Gaming mode: smaller socket buffers for lower latency at the cost of throughput.
    /// Default: false (high-throughput mode).
    #[serde(default)]
    pub gaming_mode: bool,
}

impl ListenerConfig {
    /// Returns the resolved SNI pool.
    /// Backward compat: if only `fake_sni` is set, wraps it in a pool of one.
    pub fn resolved_sni_pool(&self) -> Vec<String> {
        if !self.fake_sni_pool.is_empty() {
            return self.fake_sni_pool.clone();
        }
        if let Some(ref s) = self.fake_sni {
            return vec![s.clone()];
        }
        vec![]
    }
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
        let pool = lc.resolved_sni_pool();
        if pool.is_empty() {
            return Err(crate::error::ConfigError::MissingSni(lc.listen.to_string()));
        }
        for sni in &pool {
            if sni.len() > 219 {
                return Err(crate::error::ConfigError::SniTooLong(sni.clone()));
            }
        }
    }
    if cfg.advanced.fragmentation.fragments > 3 {
        return Err(crate::error::ConfigError::InvalidFragments);
    }
    Ok(cfg)
}
