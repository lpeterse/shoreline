use std::sync::Arc;

use tokio::time::Duration;
use super::super::{Error, Version};
use tokio::time::Instant;
use crate::link::Status;

#[derive(Debug, Clone)]
pub struct Stat {
    pub tx_bytes: u64,
    pub tx_packets: u64,
    pub rx_bytes: u64,
    pub rx_packets: u64,
    pub rx_last: Instant,
    pub status: Status,
    pub rtt: Option<Duration>,
    pub version: Option<Version>,
    pub error: Option<Arc<Error>>
}

impl Stat {
    pub fn new() -> Self {
        Self {
            tx_bytes: 0,
            tx_packets: 0,
            rx_bytes: 0,
            rx_packets: 0,
            rx_last: tokio::time::Instant::now(),
            status: Status::Init,
            rtt: None,
            version: None,
            error: None,
        }
    }

    pub fn add_tx_bytes(&mut self, n: u64) {
        self.tx_bytes = self.tx_bytes.saturating_add(n);
        self.tx_packets = self.tx_packets.saturating_add(1);
    }

    pub fn add_rx_bytes(&mut self, n: u64) {
        self.rx_bytes = self.rx_bytes.saturating_add(n);
        self.rx_packets = self.rx_packets.saturating_add(1);
    }
}
