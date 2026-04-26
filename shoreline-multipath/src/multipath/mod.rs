mod stats;
mod task;

pub use self::stats::MultiPathStats;

use self::task::MultiPathTask;
use std::collections::BTreeMap;
use std::net::SocketAddrV6;
use tokio::sync::mpsc;
use tokio::sync::watch;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
pub struct MultiPath {
    egress_tx: mpsc::Sender<Vec<u8>>,
    ingress_rx: mpsc::Receiver<Vec<u8>>,
    stats: watch::Receiver<MultiPathStats>,
    token: CancellationToken,
}

impl MultiPath {
    pub fn new(
        addrs_local: watch::Receiver<Vec<SocketAddrV6>>,
        addrs_remote: watch::Receiver<Vec<SocketAddrV6>>,
    ) -> Self {
        let (stats_tx, stats_rx) = watch::channel(MultiPathStats { paths: BTreeMap::new() });
        let token = CancellationToken::new();
        let (egress_tx, egress_rx) = mpsc::channel(100);
        let (ingress_tx, ingress_rx) = mpsc::channel(100);
        let task = MultiPathTask::new(addrs_local, addrs_remote, egress_rx, ingress_tx, stats_tx, token.clone());
        let _ = tokio::spawn(task.run());
        Self { stats: stats_rx, egress_tx, ingress_rx, token }
    }

    pub fn stats(&self) -> &watch::Receiver<MultiPathStats> {
        &self.stats
    }

    pub fn latest_rx(&self) -> Option<Instant> {
        self.stats.borrow().paths.values().filter_map(|stats| stats.borrow().latest_rx).max()
    }
}

impl Drop for MultiPath {
    fn drop(&mut self) {
        self.token.cancel();
    }
}
