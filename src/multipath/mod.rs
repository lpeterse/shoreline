mod path;
mod stats;
mod timings;
mod addr;

use self::path::Path;
use self::stats::{MultiPathStats, PathStats};
use std::collections::BTreeMap;
use std::net::SocketAddrV6;
use tokio::select;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use self::addr::SocketAddrPair;

#[derive(Debug, Clone)]
pub struct MultiPath {
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
        let task = MultiPathTask::new(addrs_local, addrs_remote, stats_tx, token.clone());
        let _ = tokio::spawn(task.run());
        Self { stats: stats_rx, token }
    }

    pub fn stats(&self) -> &watch::Receiver<MultiPathStats> {
        &self.stats
    }
}

impl Drop for MultiPath {
    fn drop(&mut self) {
        self.token.cancel();
    }
}

pub struct MultiPathTask {
    addrs_local: watch::Receiver<Vec<SocketAddrV6>>,
    addrs_remote: watch::Receiver<Vec<SocketAddrV6>>,
    paths: BTreeMap<SocketAddrPair, Path>,
    stats: watch::Sender<MultiPathStats>,
    token: CancellationToken,
}

impl MultiPathTask {
    pub fn new(
        addrs_local: watch::Receiver<Vec<SocketAddrV6>>,
        addrs_remote: watch::Receiver<Vec<SocketAddrV6>>,
        stats: watch::Sender<MultiPathStats>,
        token: CancellationToken,
    ) -> Self {
        Self { addrs_local, addrs_remote, paths: BTreeMap::new(), stats, token }
    }

    pub async fn run(mut self) {
        loop {
            select! {
                _ = self.token.cancelled() => break,
                r = self.addrs_local.changed() => {
                    match r {
                        Ok(_) => self.update_paths(),
                        Err(_) => break,
                    }
                }
                r = self.addrs_remote.changed() => {
                    match r {
                        Ok(_) => self.update_paths(),
                        Err(_) => break,
                    }
                }
            }
        }
    }

    fn update_paths(&mut self) {
        let locals = { self.addrs_local.borrow().clone() };
        let remotes = { self.addrs_remote.borrow().clone() };
        // remove paths that are no longer valid
        self.stats.send_modify(|stats| {
            // TODO
            self.paths.retain(|addr, _| locals.contains(&addr.local) && remotes.contains(&addr.remote));
            stats.paths.retain(|addr, _| locals.contains(&addr.local) && remotes.contains(&addr.remote));
            // add new paths for new local and remote addresses
            for local in &locals {
                for remote in &remotes {
                    let addr = SocketAddrPair { local: *local, remote: *remote };
                    if !self.paths.contains_key(&addr) {
                        let path = Path::new(addr, self.token.child_token());
                        self.paths.insert(addr, path);
                        stats.paths.insert(addr, PathStats::new());
                    }
                }
            }
        });
    }
}
