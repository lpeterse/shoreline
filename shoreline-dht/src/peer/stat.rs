use std::sync::Arc;

use tokio::time::Duration;
use super::super::{Error, Version, peer::Status};
use tokio::time::Instant;

#[derive(Debug, Clone, Default)]
pub struct PeerStat {
    pub tx_bytes: u64,
    pub tx_packets: u64,
    pub rx_bytes: u64,
    pub rx_packets: u64,
    pub rx_last: Option<Instant>,
    pub status: Status,
    pub rtt: Option<Duration>,
    pub version: Option<Version>,
    pub error: Option<Arc<Error>>
}

impl PeerStat {
    pub fn add_tx_bytes(&mut self, n: u64) {
        self.tx_bytes = self.tx_bytes.saturating_add(n);
        self.tx_packets = self.tx_packets.saturating_add(1);
    }

    pub fn add_rx_bytes(&mut self, n: u64) {
        self.rx_bytes = self.rx_bytes.saturating_add(n);
        self.rx_packets = self.rx_packets.saturating_add(1);
    }
}
