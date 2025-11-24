use crate::Id;
use crate::Node;
use crate::Nodes;
use crate::Peers;
use crate::peer::Peer;
use std::net::SocketAddrV6;
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::watch;
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

    pub fn peers(&self) -> impl std::ops::Deref<Target = BTreeMap<Id, Arc<Peer>>> + '_ {
        self.peers.borrow()
    }

    pub fn nodes(&self) -> impl std::ops::Deref<Target = BTreeMap<String, Arc<Node>>> + '_ {
        self.nodes.borrow()
    }
}
