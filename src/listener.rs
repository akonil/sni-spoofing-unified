use std::net::IpAddr;
use std::sync::Arc;
use std::time::Instant;

use tokio::net::TcpListener;
use tracing::{debug, error, info};

use crate::config::{FragmentationConfig, ListenerConfig, PayloadPaddingConfig};
use crate::handler;
use crate::proto::SnifferCommand;
use crate::stats::{SniTracker, Stats};

/// Simple token-bucket rate limiter (connections per second).
struct TokenBucket {
    max_tokens: f64,
    tokens: f64,
    refill_rate: f64, // tokens per millisecond
    last_refill: Instant,
}

impl TokenBucket {
    fn new(max_per_sec: u32) -> Self {
        let f = max_per_sec as f64;
        Self {
            max_tokens: f,
            tokens: f,
            refill_rate: f / 1000.0,
            last_refill: Instant::now(),
        }
    }

    fn try_acquire(&mut self) -> bool {
        let now = Instant::now();
        let elapsed_ms = now.duration_since(self.last_refill).as_millis() as f64;
        self.tokens = (self.tokens + elapsed_ms * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

pub async fn run_listener(
    lc: ListenerConfig,
    local_ip: IpAddr,
    cmd_tx: std::sync::mpsc::Sender<SnifferCommand>,
    stats: Arc<Stats>,
    sni_tracker: Arc<SniTracker>,
    handshake_timeout_ms: u64,
    confirmation_timeout_ms: u64,
    padding_cfg: PayloadPaddingConfig,
    fragmentation_cfg: FragmentationConfig,
) {
    let sni_pool = lc.resolved_sni_pool();

    let listener = match TcpListener::bind(lc.listen).await {
        Ok(l) => {
            info!(
                listen = %lc.listen,
                upstream = %lc.connect,
                sni_pool = ?sni_pool,
                gaming_mode = lc.gaming_mode,
                "listener started"
            );
            l
        }
        Err(e) => {
            error!(listen = %lc.listen, "failed to bind: {}", e);
            return;
        }
    };

    let mut bucket: Option<TokenBucket> = if lc.max_connections_per_sec > 0 {
        Some(TokenBucket::new(lc.max_connections_per_sec))
    } else {
        None
    };

    loop {
        match listener.accept().await {
            Ok((stream, peer)) => {
                if let Some(ref mut b) = bucket {
                    if !b.try_acquire() {
                        debug!(peer = %peer, "rate limit exceeded, dropping connection");
                        drop(stream);
                        continue;
                    }
                }

                let upstream = lc.connect;
                let pool = sni_pool.clone();
                let tx = cmd_tx.clone();
                let lip = local_ip;
                let gaming = lc.gaming_mode;
                let stats = Arc::clone(&stats);
                let tracker = Arc::clone(&sni_tracker);
                let hs_timeout = handshake_timeout_ms;
                let conf_timeout = confirmation_timeout_ms;
                let pad = padding_cfg.clone();
                let frag = fragmentation_cfg.clone();
                tokio::spawn(async move {
                    debug!(peer = %peer, "accepted connection");
                    handler::handle_connection(
                        stream, upstream, pool, lip, tx, gaming,
                        stats, tracker, hs_timeout, conf_timeout, pad, frag,
                    ).await;
                });
            }
            Err(e) => {
                error!("accept error: {}", e);
            }
        }
    }
}
