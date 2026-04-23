use crate::Id;
use crate::Node;
use crate::Nodes;
use crate::Peers;
use crate::peer::Peer;
use std::collections::BTreeSet;
use tokio::select;
use std::net::SocketAddrV6;
use std::ops::Deref;
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::{watch, mpsc};
use tokio_util::sync::CancellationToken;
use tokio_util::sync::DropGuard;

/// A client for the Mainline DHT network
#[derive(Debug)]
pub struct DHT {
    id: Id,
    peers: Peers,
    nodes: Nodes,
    #[allow(dead_code)]
    guard: DropGuard,
}

impl DHT {
    /// Create a new [Node] node with the given [NodeInfo]
    pub fn new(id: Id, port: u16, seeds: watch::Receiver<Vec<SocketAddrV6>>) -> Self {
        let token = CancellationToken::new();
        let peers = Peers::new(token.clone());
        let nodes = Nodes::new(id, port, peers.clone(), seeds);
        let guard = token.drop_guard();
        Self { id, peers, nodes, guard }
    }

    pub fn id(&self) -> &Id {
        &self.id
    }

    pub fn peers(&self) -> impl Deref<Target = BTreeMap<Id, Arc<Peer>>> + '_ {
        self.peers.borrow()
    }

    pub fn nodes(&self) -> impl Deref<Target = BTreeMap<String, Arc<Node>>> + '_ {
        self.nodes.borrow()
    }

    /// Search the DHT for an [Id] using A* search
    pub fn search_addrs(self: &Arc<Self>, id: &Id) -> mpsc::Receiver<SocketAddrV6> {
        let (addrs_tx, addrs_rx) = mpsc::channel(1);
        let id = *id;
        let this = Arc::clone(self);
        let _ = tokio::spawn(async move {
            let addrs_tx_ = addrs_tx.clone();
            select! {
                _ = addrs_tx_.closed() => {},
                _ = this.search_addrs_(id, addrs_tx) => {}
            }
            log::info!("DHT search for {} ended, closing address channel", id);
        });
        addrs_rx
    }

    async fn search_addrs_(&self, id: Id, addrs_tx: mpsc::Sender<SocketAddrV6>) {
        const QUEUE_LIMIT: usize = 256;

        type Metric = Id;
        type Q = BTreeMap<Metric, (Arc<Node>, SocketAddrV6)>;
        type V = BTreeSet<Id>;

        let f = |n: Arc<Peer>| {
            let metric = n.id().xor(&id);
            n.links().first_key_value().map(|(_, link)| (metric, (link.node().clone(), *link.addr())))
        };
        let ps = self.peers.borrow().clone();
        let mut v: V = ps.keys().map(|id| *id).collect();
        let mut q: Q = ps.into_values().filter_map(f).collect();

        while let Some((metric, (node, peer_addr))) = q.pop_first() {
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
            let peer_id = metric.xor(&id);
            log::info!("Searching for {}: asking peer {} at {}", id, peer_id, peer_addr);

            if peer_id == id {
                // If we find the target ID, send its address to the caller
                let _ = addrs_tx.send(peer_addr).await.ok();
            } else {
                let peer = self.peers.get(&peer_id);
                let link = peer.connect(&node, &peer_addr);
                let infos = link.find_node(&id).await.unwrap_or_default();
                for info in infos.iter() {
                    // If it's not our own ID and we haven't visited it before, add it to the queue
                    if &info.id != self.id() && v.insert(info.id) {
                        let _ = q.insert(info.id.xor(&id), (node.clone(), info.addr));
                    }
                    // Limit the queue size to prevent memory exhaustion
                    while q.len() > QUEUE_LIMIT {
                        let _ = q.pop_last();
                    }
                }
            }
        }
    }
}
