use crate::net::Netwatch;
use crate::{Node, NodeId, NodeInfo, PeerDB, net::InterfaceAddr};
use std::{collections::BTreeMap, net::SocketAddrV6, sync::Arc};
use tokio::{sync::watch, task::JoinHandle};

/// A client for the Mainline DHT network
pub struct DHT {
    id: NodeId,
    networks: watch::Receiver<BTreeMap<InterfaceAddr, Arc<Node>>>,
    task: JoinHandle<()>,
}

impl DHT {
    /// Create a new [Node] node with the given [NodeInfo]
    pub fn new<T: PeerDB>(id: NodeId, port: u16, peerdb: T) -> Self {
        let (networks_tx, networks_rx) = watch::channel(BTreeMap::new());
        let task = Task::spawn(id, port, networks_tx, peerdb);
        Self { id, networks: networks_rx, task }
    }

    pub fn id(&self) -> &NodeId {
        &self.id
    }

    pub fn nodes(&self) -> &watch::Receiver<BTreeMap<InterfaceAddr, Arc<Node>>> {
        &self.networks
    }
}

impl Drop for DHT {
    fn drop(&mut self) {
        self.task.abort();
    }
}

pub struct Task<T: PeerDB> {
    id: NodeId,
    port: u16,
    peerdb: T,
    netwatch: Netwatch,
    networks: watch::Sender<BTreeMap<InterfaceAddr, Arc<Node>>>,
}

impl<T: PeerDB> Task<T> {
    fn spawn(
        id: NodeId,
        port: u16,
        networks: watch::Sender<BTreeMap<InterfaceAddr, Arc<Node>>>,
        peerdb: T,
    ) -> JoinHandle<()> {
        let netwatch = Netwatch::new();
        let self_ = Box::new(Self { id, netwatch, networks, peerdb, port });
        tokio::spawn(self_.run())
    }

    async fn run(mut self) {
        loop {
            self.netwatch.changed().await;
            self.networks.send_modify(|current| {
                let desired = self.netwatch.list();
                current.retain(|k, _| (&desired).contains(k));
                for d in desired {
                    if !current.contains_key(&d) {
                        let addr = SocketAddrV6::new(d.addr, self.port, 0, 0);
                        let info = NodeInfo::new(self.id, addr);
                        let node = Node::new(info, self.peerdb.clone());
                        current.insert(d.clone(), Arc::new(node));
                    }
                }
            });
        }
    }
}
