use std::collections::BTreeMap;
use tokio::time::{Duration, Instant};
use tokio::sync::watch;

use crate::addr::SocketAddrPair;

#[derive(Debug, Clone)]
pub struct MultiPathStats {
    pub paths: BTreeMap<SocketAddrPair, watch::Receiver<PathStats>>,
}

#[derive(Debug, Clone)]
pub struct PathStats {
    pub rtt: Duration,
    pub jitter: Duration,
    pub latest_rx: Option<Instant>,
    pub error: Option<String>,
}

impl PathStats {
    pub fn new() -> Self {
        Self { rtt: Duration::default(), jitter: Duration::default(), latest_rx: None, error: None }
    }
}
