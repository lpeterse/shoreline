use tokio::{runtime::Runtime, sync::watch};
use crate::{config::{ConfigCtrl, ConfigState, PeerConfig}, multipath::MultiPath};
use std::{net::SocketAddrV6, sync::Arc};
use shoreline_dht::Netwatch;

#[derive(Debug, Clone)]
pub struct PeersCtrl {
    peers: watch::Receiver<Vec<Peer>>
}

impl PeersCtrl {
    pub fn new(rt: &Runtime, config: ConfigCtrl) -> Self {
        let (ps_tx, ps_rx) = watch::channel(vec![]);
        rt.spawn(async move {
            let cfg = config.subscribe();
            let mut nw = Netwatch::new();
            let mut cfg = cfg;
            let (la_tx, la_rx) = watch::channel(vec![]);

            loop {
                tokio::select! {
                    _ = nw.changed() => {
                        let addrs_local = nw.list().values().map(|ip| SocketAddrV6::new(*ip, 6882, 0, 0)).collect();
                        let _ = la_tx.send(addrs_local);
                    }
                    _ = cfg.changed() => {
                        match { cfg.borrow().clone() } {
                            ConfigState::Result(Ok(Some(config))) => {
                                let mut peers = vec![];
                                for pc in config.peers.list {
                                    peers.push(Peer::new(pc, la_rx.clone()).await);
                                }
                                let _ = ps_tx.send(peers);
                            }
                            _ => {
                                ps_tx.send(vec![]).ok();
                            },
                        };
                    }
                }
            }
        });
        Self { peers: ps_rx }
    }

    pub fn peers(&self) -> Vec<Peer> {
        self.peers.borrow().clone()
    }
}

#[derive(Debug, Clone)]
pub struct Peer {
    pub config: PeerConfig,
    pub remote_addrs: watch::Sender<Vec<SocketAddrV6>>,
    pub paths: Arc<MultiPath>,
}

impl Peer {
    pub async fn new(config: PeerConfig, local_addrs: watch::Receiver<Vec<SocketAddrV6>>) -> Self {
        let (ra_tx, ra_rx) = watch::channel(vec![]);
        let mut addrs = vec![];
        for addr in &config.addresses {
            if let Ok(addr) = addr.resolve().await {
                addrs.push(addr);
            }
        }
        let _ = ra_tx.send(addrs);
        let mp = MultiPath::new(local_addrs, ra_rx);
        Self { config, remote_addrs: ra_tx, paths: Arc::new(mp) }
    }
}
