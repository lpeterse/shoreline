use std::collections::BTreeMap;
use tokio::time::Duration;
use tokio::sync::watch;

use crate::multipath::addr::SocketAddrPair;

#[derive(Debug, Clone)]
pub struct MultiPathStats {
    pub paths: BTreeMap<SocketAddrPair, watch::Receiver<PathStats>>,
}

#[derive(Debug, Clone)]
pub struct PathStats {
    pub rtt: Duration,
    pub jitter: Duration,
    pub error: Option<String>,
}

impl PathStats {
    pub fn new() -> Self {
        Self { rtt: Duration::default(), jitter: Duration::default(), error: None }
    }
}
