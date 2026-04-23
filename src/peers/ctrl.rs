use crate::{
    config::{ConfigCtrl, ConfigState, PeerConfig}, dht::DhtCtrl, model::{HostAddress, PublicKey}
};
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use shoreline_dht::{DHT, Netwatch};
use shoreline_multipath::MultiPath;
use std::{net::SocketAddrV6, sync::Arc, time::Duration};
use tokio::{runtime::Runtime, select, sync::watch, task::JoinHandle, time::interval};

#[derive(Debug, Clone)]
pub struct PeersCtrl {
    peers: watch::Receiver<Vec<Peer>>,
}

impl PeersCtrl {
    pub fn new(rt: &Runtime, config: ConfigCtrl, dht: DhtCtrl) -> Self {
        let cfg = config.subscribe();
        let dht = dht.dht().clone();
        let (peers_tx, peers_rx) = watch::channel(vec![]);
        let task = Box::new(PeersCtrlTask::new(cfg, dht, peers_tx));
        let _ = rt.spawn(task.run());
        Self { peers: peers_rx }
    }

    pub fn peers(&self) -> Vec<Peer> {
        self.peers.borrow().clone()
    }
}

pub struct PeersCtrlTask {
    cfg: watch::Receiver<ConfigState>,
    dht: watch::Receiver<Option<Arc<DHT>>>,
    peers: watch::Sender<Vec<Peer>>,
}

impl PeersCtrlTask {
    pub fn new(
        cfg: watch::Receiver<ConfigState>,
        dht: watch::Receiver<Option<Arc<DHT>>>,
        peers: watch::Sender<Vec<Peer>>,
    ) -> Self {
        Self { cfg, dht, peers }
    }

    pub async fn run(mut self) {
        let mut nw = Netwatch::new();
        let (la_tx, la_rx) = watch::channel(vec![]);

        loop {
            select! {
                _ = nw.changed() => {
                    let addrs_local = nw.list().values().map(|ip| SocketAddrV6::new(*ip, 6882, 0, 0)).collect();
                    let _ = la_tx.send(addrs_local);
                }
                _ = self.cfg.changed() => {
                    match { self.cfg.borrow().clone() } {
                        ConfigState::Result(Ok(Some(config))) => {
                            let mut peers = vec![];
                            for pc in config.peers.list {
                                peers.push(Peer::new(pc, la_rx.clone(), self.dht.clone()));
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
    ) -> Self {
        let (ra_tx, ra_rx) = watch::channel(vec![]);
        let mp = Arc::new(MultiPath::new(la_rx, ra_rx));
        let task = PeerTask::new(config.pubkey.clone(), mp.clone(), dht, ra_tx, config.addresses.clone());
        let task = tokio::spawn(task.run());
        Self { config, paths: mp, task: Arc::new(task) }
    }
}

pub struct PeerTask {
    pubkey: PublicKey,
    multipath: Arc<MultiPath>,
    dht_rx: watch::Receiver<Option<Arc<DHT>>>,
    ra_tx: watch::Sender<Vec<SocketAddrV6>>,
    ra_static: Vec<HostAddress>,

    mdns: ServiceDaemon,
}

impl PeerTask {
    pub fn new(pubkey: PublicKey, multipath: Arc<MultiPath>, dht_rx: watch::Receiver<Option<Arc<DHT>>>, ra_tx: watch::Sender<Vec<SocketAddrV6>>, ra_static: Vec<HostAddress>) -> Self {
        Self { pubkey, multipath, dht_rx, ra_tx, ra_static, mdns: ServiceDaemon::new().expect("Failed to create mDNS service daemon") }
    }

    pub async fn run(self) {
        let id = self.pubkey.to_dht_id();
        let service_type = "_shoreline._udp.local.";
        let name = format!("{}", id);
        let hostname = format!("{}.local.", id);
        let port = 6882;
        let properties = [("property_1", "test"), ("property_2", "test")];
        let info = ServiceInfo::new(service_type, &name, &hostname, (), port, &properties[..]).expect("Failed to create mDNS service info");
        let mut info = info.enable_addr_auto();
        info.set_link_local_only(true);
        self.mdns.register(info).expect("Failed to register mDNS service");

        // Browse for a service type.
        let receiver = self.mdns.browse(service_type).expect("Failed to browse");

        // Receive the browse events in sync or async. Here is
        // an example of using a thread. Users can call `receiver.recv_async().await`
        // if running in async environment.
        std::thread::spawn(move || {
            while let Ok(event) = receiver.recv() {
                match event {
                    ServiceEvent::ServiceResolved(resolved) => {
                        println!("Resolved a new service: {:?}", resolved.addresses);
                    }
                    other_event => {
                        println!("Received other event: {:?}", &other_event);
                    }
                }
            }
        });

        std::future::pending::<()>().await;
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
