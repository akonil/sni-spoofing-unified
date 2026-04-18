use std::net::IpAddr;

use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnId {
    pub src_ip: IpAddr,
    pub src_port: u16,
    pub dst_ip: IpAddr,
    pub dst_port: u16,
}

#[derive(Debug)]
pub enum SnifferResult {
    /// Fake ClientHello was ignored by the server (DPI bypass succeeded).
    /// Carries the SNI that was used, for tracking.
    FakeConfirmed { sni: String },
    Failed(String),
}

pub struct Registration {
    pub conn_id: ConnId,
    /// One or more fragments of the fake ClientHello payload.
    /// Single-element vec = no fragmentation (normal case).
    pub fake_payloads: Vec<Vec<u8>>,
    /// The SNI chosen for this connection (for tracking).
    pub fake_sni: String,
    /// Delay between fragments in milliseconds (0 = no delay).
    pub fragment_delay_ms: u64,
    pub result_tx: mpsc::Sender<SnifferResult>,
    pub registered_tx: tokio::sync::oneshot::Sender<()>,
}

pub struct Deregistration {
    pub conn_id: ConnId,
}

pub enum SnifferCommand {
    Register(Registration),
    Deregister(Deregistration),
}
