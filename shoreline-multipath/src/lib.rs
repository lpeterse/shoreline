mod addr;
mod path;
mod stats;
mod timings;
mod constants;

use self::addr::SocketAddrPair;
use self::path::Path;
use self::stats::MultiPathStats;
use std::collections::BTreeMap;
use std::net::SocketAddrV6;
use tokio::select;
use tokio::sync::watch;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

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

    pub fn latest_rx(&self) -> Option<Instant> {
        self.stats.borrow().paths.values().filter_map(|stats| stats.borrow().latest_rx).max()
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
                    if Self::is_valid_pair(&addr) && !self.paths.contains_key(&addr) {
                        let path = Path::new(addr, self.token.child_token());
                        stats.paths.insert(addr, path.stats().clone());
                        self.paths.insert(addr, path);
                    }
                }
            }
        });
    }

    fn is_valid_pair(pair: &SocketAddrPair) -> bool {
        if pair.local.scope_id() != pair.remote.scope_id() {
            return false;
        }
        if Self::is_gua(pair.local.ip()) != Self::is_gua(pair.remote.ip()) {
            return false;
        }
        if Self::is_ula(pair.local.ip()) != Self::is_ula(pair.remote.ip()) {
            return false;
        }
        if Self::is_lla(pair.local.ip()) != Self::is_lla(pair.remote.ip()) {
            return false;
        }
        true
    }

    /// Check if the address is a global unicast address
    fn is_gua(ip: &std::net::Ipv6Addr) -> bool {
        !(ip.is_loopback()
            || ip.is_unspecified()
            || ip.is_multicast()
            || ip.is_unique_local()
            || ip.is_unicast_link_local())
    }

    /// Check if the address is a unique local address
    fn is_ula(ip: &std::net::Ipv6Addr) -> bool {
        ip.is_unique_local()
    }

    /// Check if the address is a link local address
    fn is_lla(ip: &std::net::Ipv6Addr) -> bool {
        ip.is_unicast_link_local()
    }
}
