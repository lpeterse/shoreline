use std::sync::Arc;

use super::super::Error;

#[derive(Debug, Clone, Default)]
pub struct NodeStat {
    pub tx_bytes: u64,
    pub tx_packets: u64,
    pub rx_bytes: u64,
    pub rx_packets: u64,
    pub error: Option<Arc<Error>>
}

impl NodeStat {
    pub fn add_tx_bytes(&mut self, n: u64) {
        self.tx_bytes = self.tx_bytes.saturating_add(n);
    }

    pub fn add_tx_packets(&mut self, n: u64) {
        self.tx_packets = self.tx_packets.saturating_add(n);
    }

    pub fn add_rx_bytes(&mut self, n: u64) {
        self.rx_bytes = self.rx_bytes.saturating_add(n);
    }

    pub fn add_rx_packets(&mut self, n: u64) {
        self.rx_packets = self.rx_packets.saturating_add(n);
    }

    pub fn set_error(&mut self, e: Option<Arc<Error>>) {
        self.error = e;
    }
}
