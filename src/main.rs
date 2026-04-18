#![allow(dead_code)]

mod config;
mod debounce;
mod error;
mod handler;
mod listener;
mod packet;
mod proto;
mod relay;
mod shutdown;
mod sniffer;
mod stats;
mod wizard;

use std::net::IpAddr;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use tracing::{error, info};
use tracing_subscriber::EnvFilter;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .init();

    let args: Vec<String> = std::env::args().collect();

    // --wizard: interactive first-run config generator
    if args.iter().any(|a| a == "--wizard") {
        if let Err(e) = wizard::run_wizard() {
            error!("{}", e);
            std::process::exit(1);
        }
        return;
    }

    // --preset <name>: generate config from a named preset
    if let Some(pos) = args.iter().position(|a| a == "--preset") {
        let name = args.get(pos + 1).map(|s| s.as_str()).unwrap_or("");
        if let Err(e) = wizard::apply_preset(name) {
            error!("{}", e);
            std::process::exit(1);
        }
        return;
    }

    let config_path = args.get(1)
        .filter(|a| !a.starts_with("--"))
        .cloned()
        .unwrap_or_else(|| "config.json".into());

    let cfg = match config::load(&config_path) {
        Ok(c) => c,
        Err(e) => {
            error!("{}", e);
            std::process::exit(1);
        }
    };

    debounce::set_enabled(cfg.debounce_logs);

    let upstream_addrs: Vec<(IpAddr, u16)> = cfg
        .listeners
        .iter()
        .map(|lc| (lc.connect.ip(), lc.connect.port()))
        .collect();

    let local_ips: Vec<IpAddr> = upstream_addrs
        .iter()
        .filter_map(|(ip, _)| resolve_local_ip(*ip).ok())
        .collect();

    if local_ips.is_empty() {
        error!("could not determine local IP for any upstream");
        std::process::exit(1);
    }

    info!(
        "config loaded: {} listener(s), local IPs: {:?}",
        cfg.listeners.len(),
        local_ips
    );

    let upstream_sockaddrs: Vec<std::net::SocketAddr> =
        cfg.listeners.iter().map(|lc| lc.connect).collect();

    #[cfg(target_os = "linux")]
    let backend = match sniffer::linux::AfPacketBackend::open(&upstream_sockaddrs) {
        Ok(b) => b,
        Err(e) => {
            error!("failed to open raw socket: {}", e);
            error!("hint: run with sudo or CAP_NET_RAW");
            std::process::exit(1);
        }
    };

    #[cfg(target_os = "macos")]
    let backend = match sniffer::macos::BpfBackend::open(&upstream_sockaddrs) {
        Ok(b) => b,
        Err(e) => {
            error!("failed to open BPF device: {}", e);
            error!("hint: run with sudo");
            std::process::exit(1);
        }
    };

    #[cfg(target_os = "windows")]
    let backend = match sniffer::windows::WinDivertBackend::open(&upstream_sockaddrs) {
        Ok(b) => b,
        Err(e) => {
            error!("failed to open WinDivert: {}", e);
            error!("hint: run as Administrator");
            std::process::exit(1);
        }
    };

    let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<proto::SnifferCommand>();

    let stop = Arc::new(AtomicBool::new(false));

    let jitter = cfg.jitter.clone();
    info!(
        jitter_min_ms = jitter.min_ms,
        jitter_max_ms = jitter.max_ms,
        jitter_enabled = !jitter.is_disabled(),
        "inject jitter configured"
    );

    let sniffer_stop = stop.clone();
    let sniffer_local_ips = local_ips.clone();
    let sniffer_upstreams = upstream_addrs.clone();
    std::thread::Builder::new()
        .name("sniffer".into())
        .spawn(move || {
            sniffer::run_sniffer(backend, cmd_rx, sniffer_local_ips, sniffer_upstreams, sniffer_stop, jitter);
        })
        .expect("failed to spawn sniffer thread");

    let sni_tracker = stats::SniTracker::new();

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    rt.block_on(async {
        let signal_stop = stop.clone();
        tokio::spawn(async move {
            shutdown::wait_for_signal(signal_stop).await;
            tokio::time::sleep(Duration::from_secs(1)).await;
            std::process::exit(0);
        });

        // Log per-SNI stats every 60 seconds.
        let tracker_log = Arc::clone(&sni_tracker);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                let snapshot = tracker_log.snapshot();
                if !snapshot.is_empty() {
                    info!("SNI stats (top 10 by success):");
                    for (sni, ok, fail) in snapshot.iter().take(10) {
                        info!("  {} → ok={} fail={}", sni, ok, fail);
                    }
                }
            }
        });

        let padding_cfg = cfg.advanced.payload_padding.clone();
        let fragmentation_cfg = cfg.advanced.fragmentation.clone();

        let mut handles = Vec::new();
        for lc in cfg.listeners {
            let tx = cmd_tx.clone();
            let lip = resolve_local_ip(lc.connect.ip()).unwrap_or(local_ips[0]);
            let listener_stats = stats::Stats::new();
            let tracker = Arc::clone(&sni_tracker);
            handles.push(tokio::spawn(listener::run_listener(
                lc,
                lip,
                tx,
                listener_stats,
                tracker,
                cfg.timeouts.handshake_timeout_ms,
                cfg.timeouts.confirmation_timeout_ms,
                padding_cfg.clone(),
                fragmentation_cfg.clone(),
            )));
        }

        for h in handles {
            let _ = h.await;
        }
    });
}

fn resolve_local_ip(dst: IpAddr) -> Result<IpAddr, String> {
    use std::net::UdpSocket;

    let target = match dst {
        IpAddr::V4(v4) => format!("{}:53", v4),
        IpAddr::V6(v6) => format!("[{}]:53", v6),
    };
    let bind = if dst.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" };

    let sock = UdpSocket::bind(bind).map_err(|e| format!("bind: {}", e))?;
    sock.connect(&target).map_err(|e| format!("connect: {}", e))?;
    Ok(sock.local_addr().map_err(|e| format!("local_addr: {}", e))?.ip())
}
