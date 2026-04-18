use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use socket2::{SockRef, TcpKeepalive};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tracing::{debug, info};

use crate::config::{FragmentationConfig, PayloadPaddingConfig};
use crate::error::HandlerError;
use crate::log_debounced;
use crate::packet::tls;
use crate::proto::{ConnId, Deregistration, Registration, SnifferCommand, SnifferResult};
use crate::relay;
use crate::stats::{ConnectionGuard, SniTracker, Stats};

pub async fn handle_connection(
    client: TcpStream,
    upstream_addr: SocketAddr,
    sni_pool: Vec<String>,
    local_ip: std::net::IpAddr,
    cmd_tx: std::sync::mpsc::Sender<SnifferCommand>,
    gaming_mode: bool,
    stats: Arc<Stats>,
    sni_tracker: Arc<SniTracker>,
    handshake_timeout_ms: u64,
    confirmation_timeout_ms: u64,
    padding_cfg: PayloadPaddingConfig,
    fragmentation_cfg: FragmentationConfig,
) {
    // Guard increments active+total on creation, decrements active on drop.
    let guard = ConnectionGuard::new(stats.clone());
    let (active, total) = guard.snapshot();
    info!(upstream = %upstream_addr, active, total, "connection opened");

    let result = handle_inner(
        client,
        upstream_addr,
        sni_pool,
        local_ip,
        &cmd_tx,
        gaming_mode,
        &sni_tracker,
        handshake_timeout_ms,
        confirmation_timeout_ms,
        &padding_cfg,
        &fragmentation_cfg,
    ).await;

    if let Err(ref e) = result {
        match e {
            HandlerError::Timeout => {
                log_debounced!("handler_timeout", warn, upstream = %upstream_addr, "timeout waiting for fake ACK");
            }
            _ => {
                log_debounced!("handler_error", warn, upstream = %upstream_addr, "connection failed: {}", e);
            }
        }
    }

    // Drop guard first so active count reflects the closed connection.
    let total = guard.snapshot().1;
    drop(guard);
    let active = stats.snapshot().0;
    info!(upstream = %upstream_addr, active, total, "connection closed");
}

/// Configure a socket for either gaming (low-latency) or throughput mode.
fn configure_socket(sock_ref: &socket2::SockRef, gaming_mode: bool) {
    // Always enable TCP_NODELAY to disable Nagle's algorithm
    let _ = sock_ref.set_nodelay(true);

    if gaming_mode {
        // Small buffers → less queuing, lower latency
        let _ = sock_ref.set_recv_buffer_size(32_768);
        let _ = sock_ref.set_send_buffer_size(32_768);
    } else {
        // Large buffers → higher throughput
        let _ = sock_ref.set_recv_buffer_size(262_144);
        let _ = sock_ref.set_send_buffer_size(262_144);
    }
}

fn split_payload(payload: &[u8], n: usize) -> Vec<Vec<u8>> {
    let chunk = (payload.len() + n - 1) / n;
    payload.chunks(chunk).map(|c| c.to_vec()).collect()
}

#[allow(clippy::too_many_arguments)]
async fn handle_inner(
    client: TcpStream,
    upstream_addr: SocketAddr,
    sni_pool: Vec<String>,
    local_ip: std::net::IpAddr,
    cmd_tx: &std::sync::mpsc::Sender<SnifferCommand>,
    gaming_mode: bool,
    sni_tracker: &Arc<SniTracker>,
    handshake_timeout_ms: u64,
    confirmation_timeout_ms: u64,
    padding_cfg: &PayloadPaddingConfig,
    fragmentation_cfg: &FragmentationConfig,
) -> Result<(), HandlerError> {
    // Pick a fake SNI from the pool (random if pool has multiple entries).
    let fake_sni = if sni_pool.len() == 1 {
        sni_pool[0].clone()
    } else {
        use rand::Rng;
        let idx = rand::thread_rng().gen_range(0..sni_pool.len());
        sni_pool[idx].clone()
    };

    // Build the fake ClientHello (with optional random padding).
    let full_payload = tls::build_client_hello_padded(&fake_sni, padding_cfg);

    // Optionally split into N fragments.
    let fake_payloads = if fragmentation_cfg.enabled && fragmentation_cfg.fragments > 1 {
        split_payload(&full_payload, fragmentation_cfg.fragments)
    } else {
        vec![full_payload]
    };

    let upstream_sock = if upstream_addr.is_ipv4() {
        socket2::Socket::new(
            socket2::Domain::IPV4,
            socket2::Type::STREAM,
            Some(socket2::Protocol::TCP),
        )
    } else {
        socket2::Socket::new(
            socket2::Domain::IPV6,
            socket2::Type::STREAM,
            Some(socket2::Protocol::TCP),
        )
    }
    .map_err(HandlerError::Connect)?;

    upstream_sock.set_nonblocking(true).map_err(HandlerError::Connect)?;

    let bind_addr: SocketAddr = if upstream_addr.is_ipv4() {
        "0.0.0.0:0".parse().unwrap()
    } else {
        "[::]:0".parse().unwrap()
    };
    upstream_sock
        .bind(&bind_addr.into())
        .map_err(HandlerError::Connect)?;

    let local_addr = upstream_sock
        .local_addr()
        .map_err(HandlerError::Connect)?
        .as_socket()
        .ok_or_else(|| HandlerError::Connect(std::io::Error::new(
            std::io::ErrorKind::Other,
            "failed to get local socket addr",
        )))?;

    let (result_tx, mut result_rx) = mpsc::channel::<SnifferResult>(4);

    let conn_id = ConnId {
        src_ip: local_ip,
        src_port: local_addr.port(),
        dst_ip: upstream_addr.ip(),
        dst_port: upstream_addr.port(),
    };

    let (registered_tx, registered_rx) = tokio::sync::oneshot::channel();
    cmd_tx
        .send(SnifferCommand::Register(Registration {
            conn_id,
            fake_payloads,
            fake_sni: fake_sni.clone(),
            fragment_delay_ms: fragmentation_cfg.delay_ms,
            result_tx,
            registered_tx,
        }))
        .map_err(|_| HandlerError::Registration)?;

    let _ = registered_rx.await;

    match upstream_sock.connect(&upstream_addr.into()) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
        #[cfg(unix)]
        Err(e) if e.raw_os_error() == Some(libc::EINPROGRESS) => {}
        Err(e) => {
            let _ = cmd_tx.send(SnifferCommand::Deregister(Deregistration { conn_id }));
            return Err(HandlerError::Connect(e));
        }
    }

    let std_stream: std::net::TcpStream = upstream_sock.into();
    let upstream = TcpStream::from_std(std_stream).map_err(HandlerError::Connect)?;

    let connect_result = tokio::time::timeout(Duration::from_millis(handshake_timeout_ms), upstream.writable()).await;
    match connect_result {
        Ok(Ok(())) => {
            let sock_ref = SockRef::from(&upstream);
            if let Some(err) = sock_ref.take_error().map_err(HandlerError::Connect)? {
                let _ = cmd_tx.send(SnifferCommand::Deregister(Deregistration { conn_id }));
                return Err(HandlerError::Connect(err));
            }
        }
        Ok(Err(e)) => {
            let _ = cmd_tx.send(SnifferCommand::Deregister(Deregistration { conn_id }));
            return Err(HandlerError::Connect(e));
        }
        Err(_) => {
            let _ = cmd_tx.send(SnifferCommand::Deregister(Deregistration { conn_id }));
            return Err(HandlerError::Connect(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "connect timeout",
            )));
        }
    }

    let keepalive = TcpKeepalive::new()
        .with_time(Duration::from_secs(11))
        .with_interval(Duration::from_secs(2));

    let sock_ref = SockRef::from(&upstream);
    let _ = sock_ref.set_tcp_keepalive(&keepalive);
    configure_socket(&sock_ref, gaming_mode);

    let client_ref = SockRef::from(&client);
    let _ = client_ref.set_tcp_keepalive(&keepalive);
    configure_socket(&client_ref, gaming_mode);

    debug!(port = local_addr.port(), "connected, waiting for sniffer confirmation");

    let confirmed = tokio::time::timeout(Duration::from_millis(confirmation_timeout_ms), async {
        while let Some(result) = result_rx.recv().await {
            match result {
                SnifferResult::FakeConfirmed { sni } => {
                    sni_tracker.record_success(&sni);
                    return Ok(());
                }
                SnifferResult::Failed(e) => {
                    sni_tracker.record_failure(&fake_sni);
                    return Err(HandlerError::SnifferFailed(e));
                }
            }
        }
        Err(HandlerError::Registration)
    })
    .await;

    match confirmed {
        Ok(Ok(())) => {}
        Ok(Err(e)) => return Err(e),
        Err(_) => {
            sni_tracker.record_failure(&fake_sni);
            return Err(HandlerError::Timeout);
        }
    }

    info!(port = local_addr.port(), "fake confirmed, starting relay");

    relay::relay(client, upstream).await.map_err(HandlerError::Relay)
}
