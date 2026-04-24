use crate::config::{ConfigCtrl, ConfigState, PeerConfig};
use crate::dht::DhtCtrl;
use crate::model::{HostAddress, PublicKey};
use crate::util::NetworkInterface;
use shoreline_dht::{DHT};
use crate::util::Netwatch;
use shoreline_multipath::MultiPath;
use std::collections::BTreeMap;
use std::{net::SocketAddrV6};
use std::sync::Arc;
use crate::mdns::{MdnsCtrl, MdnsEntry};
use tokio::runtime::Runtime;
use tokio::select;
use tokio::sync::watch;
use tokio::task::JoinHandle;

#[derive(Debug, Clone)]
pub struct PeersCtrl {
    peers: watch::Receiver<Vec<Peer>>,
    netwatch: Netwatch,
}

impl PeersCtrl {
    pub fn new(rt: &Runtime, config: ConfigCtrl, dht: DhtCtrl, mdns: MdnsCtrl) -> Self {
        let netwatch = Netwatch::new(rt);
        let cfg = config.subscribe();
        let dht = dht.dht().clone();
        let mdns = mdns.entries_().clone();
        let (peers_tx, peers_rx) = watch::channel(vec![]);
        let task = Box::new(PeersCtrlTask::new(cfg, dht, mdns, peers_tx));
        let _ = rt.spawn(task.run(netwatch.clone()));
        Self { peers: peers_rx, netwatch }
    }

    pub fn peers(&self) -> Vec<Peer> {
        self.peers.borrow().clone()
    }

    pub fn interfaces(&self) -> Vec<NetworkInterface> {
        self.netwatch.list()
    }
}

pub struct PeersCtrlTask {
    cfg: watch::Receiver<ConfigState>,
    dht: watch::Receiver<Option<Arc<DHT>>>,
    mdns: watch::Receiver<BTreeMap<PublicKey, MdnsEntry>>,
    peers: watch::Sender<Vec<Peer>>,
}

impl PeersCtrlTask {
    pub fn new(
        cfg: watch::Receiver<ConfigState>,
        dht: watch::Receiver<Option<Arc<DHT>>>,
        mdns: watch::Receiver<BTreeMap<PublicKey, MdnsEntry>>,
        peers: watch::Sender<Vec<Peer>>,
    ) -> Self {
        Self { cfg, dht, mdns, peers }
    }

    pub async fn run(mut self, mut nw: Netwatch) {
        let (la_tx, la_rx) = watch::channel(vec![]);

        loop {
            select! {
                _ = nw.changed() => {
                    let mut addrs = vec![];
                    for addr in nw.list().into_iter() {
                        for ip in &addr.addrs {
                            addrs.push(SocketAddrV6::new(*ip, 6882, 0, addr.index));
                        }
                    }
                    let _ = la_tx.send(addrs);
                }
                _ = self.cfg.changed() => {
                    match { self.cfg.borrow().clone() } {
                        ConfigState::Result(Ok(Some(config))) => {
                            let mut peers = vec![];
                            for pc in config.peers.list {
                                peers.push(Peer::new(pc, la_rx.clone(), self.dht.clone(), self.mdns.clone()));
                            }
                            let _ = self.peers.send(peers);
                        }
                        _ => {
                            self.peers.send(vec![]).ok();
                        },
                    };
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Peer {
    pub config: PeerConfig,
    pub paths: Arc<MultiPath>,
    pub task: Arc<JoinHandle<()>>,
}

impl Peer {
    pub fn new(
        config: PeerConfig,
        la_rx: watch::Receiver<Vec<SocketAddrV6>>,
        dht: watch::Receiver<Option<Arc<DHT>>>,
        mdns: watch::Receiver<BTreeMap<PublicKey, MdnsEntry>>,
    ) -> Self {
        let (ra_tx, ra_rx) = watch::channel(vec![]);
        let mp = Arc::new(MultiPath::new(la_rx, ra_rx));
        let task = PeerTask::new(config.pubkey.clone(), mp.clone(), dht, mdns, ra_tx, config.addresses.clone());
        let task = tokio::spawn(task.run());
        Self { config, paths: mp, task: Arc::new(task) }
    }
}

pub struct PeerTask {
    pubkey: PublicKey,
    multipath: Arc<MultiPath>,
    dht_rx: watch::Receiver<Option<Arc<DHT>>>,
    mdns_rx: watch::Receiver<BTreeMap<PublicKey, MdnsEntry>>,
    ra_tx: watch::Sender<Vec<SocketAddrV6>>,
    ra_static: Vec<HostAddress>,
}

impl PeerTask {
    pub fn new(pubkey: PublicKey, multipath: Arc<MultiPath>, dht_rx: watch::Receiver<Option<Arc<DHT>>>, mdns_rx: watch::Receiver<BTreeMap<PublicKey, MdnsEntry>>, ra_tx: watch::Sender<Vec<SocketAddrV6>>, ra_static: Vec<HostAddress>) -> Self {
        Self { pubkey, multipath, dht_rx, mdns_rx, ra_tx, ra_static }
    }

    pub async fn run(mut self) {
        loop {
            select! {
                _ = self.mdns_rx.changed() => {
                    if let Some(entry) = self.mdns_rx.borrow().get(&self.pubkey).cloned() {
                        let mut addrs = vec![];
                        for addr in &entry.addrs {
                            addrs.push(SocketAddrV6::new(*addr.addr(), entry.port, 0, addr.scope_id().index));
                        }
                        self.ra_tx.send(addrs.to_vec()).ok();
                    } else {
                        self.ra_tx.send(vec![]).ok();
                    }
                }
            }
        }
    }

    async fn update_addrs(&self) {
        let dht = self.dht_rx.borrow().clone();
        let resolved = self.resolve_static().await;
        let _ = self.ra_tx.send(resolved);

        if let Some(dht) = &dht {
            let mut addrs_rx = dht.search_addrs(&self.pubkey.to_dht_id());
            while let Some(mut addr) = addrs_rx.recv().await {
                addr.set_port(6882);
                self.ra_tx.send_modify(|addrs|addrs.push(addr));
            }
        }
    }

    async fn resolve_static(&self) -> Vec<SocketAddrV6> {
        let mut addrs = vec![];
        for addr in &self.ra_static {
            if let Ok(addr) = addr.resolve().await {
                addrs.push(addr);
            }
        }
        addrs
    }
}
