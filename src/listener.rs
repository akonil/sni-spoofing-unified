use std::net::IpAddr;
use std::sync::Arc;

use tokio::net::TcpListener;
use tracing::{error, info};

use crate::config::ListenerConfig;
use crate::handler;
use crate::proto::SnifferCommand;
use crate::stats::Stats;

pub async fn run_listener(
    lc: ListenerConfig,
    local_ip: IpAddr,
    cmd_tx: std::sync::mpsc::Sender<SnifferCommand>,
    stats: Arc<Stats>,
    handshake_timeout_ms: u64,
    confirmation_timeout_ms: u64,
) {
    let listener = match TcpListener::bind(lc.listen).await {
        Ok(l) => {
            info!(
                listen = %lc.listen,
                upstream = %lc.connect,
                sni = %lc.fake_sni,
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

    loop {
        match listener.accept().await {
            Ok((stream, peer)) => {
                let upstream = lc.connect;
                let sni = lc.fake_sni.clone();
                let tx = cmd_tx.clone();
                let lip = local_ip;
                let gaming = lc.gaming_mode;
                let stats = Arc::clone(&stats);
                let hs_timeout = handshake_timeout_ms;
                let conf_timeout = confirmation_timeout_ms;
                tokio::spawn(async move {
                    tracing::debug!(peer = %peer, "accepted connection");
                    handler::handle_connection(stream, upstream, sni, lip, tx, gaming, stats, hs_timeout, conf_timeout).await;
                });
            }
            Err(e) => {
                error!("accept error: {}", e);
            }
        }
    }
}
