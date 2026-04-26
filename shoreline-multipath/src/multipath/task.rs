use crate::PathAddr;
use crate::path::Path;
use super::stats::MultiPathStats;
use tokio::sync::mpsc;
use std::collections::BTreeMap;
use std::net::SocketAddrV6;
use tokio::select;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

pub struct MultiPathTask {
    addrs_local: watch::Receiver<Vec<SocketAddrV6>>,
    addrs_remote: watch::Receiver<Vec<SocketAddrV6>>,
    egress_rx: mpsc::Receiver<Vec<u8>>,
    ingress_tx: mpsc::Sender<Vec<u8>>,
    paths: BTreeMap<PathAddr, Path>,
    stats: watch::Sender<MultiPathStats>,
    token: CancellationToken,
}

impl MultiPathTask {
    pub fn new(
        addrs_local: watch::Receiver<Vec<SocketAddrV6>>,
        addrs_remote: watch::Receiver<Vec<SocketAddrV6>>,
        egress_rx: mpsc::Receiver<Vec<u8>>,
        ingress_tx: mpsc::Sender<Vec<u8>>,
        stats: watch::Sender<MultiPathStats>,
        token: CancellationToken,
    ) -> Self {
        Self { addrs_local, addrs_remote, egress_rx, ingress_tx, paths: BTreeMap::new(), stats, token }
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
            // Remote addresses are considered equal if they have the same IP and port
            let g = |addr1: &SocketAddrV6, addr2: &SocketAddrV6| {
                addr1.ip() == addr2.ip() && addr1.port() == addr2.port()
            };
            // Retain paths that are a combination of current local and remote addresses
            let f = |addr: &PathAddr| {
                let local_valid = locals.contains(&addr.local());
                let remote_valid = remotes.iter().any(|x| g(x, &addr.remote()));
                local_valid && remote_valid
            };
            self.paths.retain(|addr, _| f(addr));
            stats.paths.retain(|addr, _| f(addr));
            // Add new paths for new local and remote addresses
            for local in &locals {
                for remote in &remotes {
                    // Only pair addresses that are in the same scope (e.g., both link-local or both global)
                    // The remote scope id is set to match the local scope id
                    if let Some(addr) = PathAddr::new(*local, *remote) {
                        if !self.paths.contains_key(&addr) {
                            if let Ok(path) = Path::new(addr, self.token.child_token()) {
                                stats.paths.insert(addr, path.stats().clone());
                                self.paths.insert(addr, path);
                            }
                        }
                    }
                }
            }
        });
    }

    fn best_path(&self) -> Option<&Path> {
        self.paths.values().filter(|path| path.status().borrow().is_open()).min_by_key(|path| path.stats().borrow().rtt)
    }
}
