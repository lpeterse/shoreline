mod cmd;
mod peerdb;
mod socket;
mod stat;
mod table;
mod task;

use self::task::NodeTask;
use super::{Error, Peer, PeerId, PeerInfo, Version, NodeId, NodeInfo};
use std::collections::{BTreeMap, BTreeSet};
use std::net::SocketAddrV6;
use std::sync::{Arc, Weak};
use tokio::sync::mpsc;
use tokio::sync::{oneshot, watch};
use tokio::task::JoinHandle;

pub use self::cmd::NodeCmd;
pub use self::peerdb::PeerDB;
pub use self::stat::NodeStat;

/// A client for the Mainline DHT network
pub struct Node {
    info: NodeInfo,
    cmds: mpsc::UnboundedSender<NodeCmd>,
    stat: watch::Receiver<NodeStat>,
    peers: watch::Receiver<BTreeMap<PeerId, Weak<Peer>>>,
    task: JoinHandle<()>,
}

impl Node {
    /// Create a new [Node] node with the given [NodeInfo]
    pub fn new<T: PeerDB>(info: NodeInfo, peerdb: T) -> Self {
        let (stat_, stat) = watch::channel(NodeStat::default());
        let (peers_, peers) = watch::channel(BTreeMap::new());
        let cmds = mpsc::unbounded_channel();
        let cmdr = cmds.1;
        let cmds = cmds.0;
        let cmdw = cmds.downgrade();
        let task = NodeTask::spawn(info, stat_, cmdr, cmdw, peerdb, peers_);
        Self { info, cmds, task, stat, peers }
    }

    /// Get this node's [NodeId]
    pub fn id(&self) -> &NodeId {
        &self.info.id
    }

    /// Get this node's [AddressWatch]
    pub fn addr(&self) -> &SocketAddrV6 {
        &self.info.addr
    }

    /// Get this node's [Version] (a.k.a. client identifier)
    pub fn version(&self) -> Version {
        Version::SELF
    }

    pub fn error(&self) -> Option<Arc<Error>> {
        self.stat.borrow().error.clone()
    }

    pub fn stat(&self) -> NodeStat {
        self.stat.borrow().clone()
    }

    /// Suggest to connect to a [Peer] and eventually add it to the table
    ///
    /// A node is added to the table if it is responsive and either fills a
    /// gap in the routing table or is a better candidate than an existing node
    /// (e.g. has lower RTT).
    pub fn suggest(&self, info: &PeerInfo) -> Result<(), Error> {
        self.cmds.send(NodeCmd::SuggestNode(info.clone())).map_err(|_| Error::NodeTerminated)
    }

    /// Search the DHT for a [PeerId] using A* search
    pub async fn search(&self, id: &PeerId) -> Result<PeerInfo, Error> {
        type Q = BTreeMap<PeerId, PeerInfo>;
        type V = BTreeSet<PeerId>;
        let f = |n: Arc<Peer>| (n.id().xor(id), PeerInfo::new(*n.id(), *n.address()));
        let m = self.peers().borrow();
        let mut v: V = m.keys().map(|d| d.xor(self.id())).collect();
        let mut q: Q = m.values().filter_map(Weak::upgrade).map(f).collect();

        while let Some((_, info)) = q.pop_first() {
            if &info.id == id {
                return Ok(info);
            }
            let n = self.peer(&info).await?;
            if let Ok(ns) = n.find_node(id).await {
                for info in ns.iter() {
                    if &info.id == id {
                        return Ok(*info);
                    }
                    if &info.id != self.id() && v.insert(info.id) {
                        q.insert(info.id.xor(id), *info);
                    }
                }
            }
        }

        Err(format!("Asked {} peers: not found", v.len()).into())
    }

    /// Lookup or create a [Peer] by [PeerInfo]
    ///
    /// Either returns an existing instance or creates a new one using the given
    /// [PeerId] and [SocketAddrV6]. The node might be added to the routing
    /// table if considered appropriate.
    pub async fn peer(&self, info: &PeerInfo) -> Result<Arc<Peer>, Error> {
        let (tx, rx) = oneshot::channel();
        self.cmds.send(NodeCmd::GetNode(info.clone(), tx)).map_err(|_| Error::NodeTerminated)?;
        rx.await.map_err(|_| Error::NodeTerminated)
    }

    /// Get a list of known [Peer]s
    ///
    /// This includes peers not necessarily in the routing table. Such peers might
    /// exist because e.g. a search process has a strong handle on it.
    pub fn peers(&self) -> &watch::Receiver<BTreeMap<PeerId, Weak<Peer>>> {
        &self.peers
    }

    /// Find [PeerInfo]s close to the given [PeerId]
    ///
    /// Currently returns a list of up to 8 peers closest to the given id.
    /// This does not perform any network operations, but is just a lookup in the routing table.
    pub async fn find(&self, id: &PeerId) -> Result<Vec<PeerInfo>, Error> {
        let (tx, rx) = oneshot::channel();
        self.cmds.send(NodeCmd::FindNode(*id, tx)).map_err(|_| Error::NodeTerminated)?;
        rx.await.map(Into::into).map_err(|_| Error::NodeTerminated)
    }

    /// Remove a [Peer] from the routing table
    ///
    /// The peer not being existent (anymore) is not considered an error.
    pub fn remove(&self, id: &PeerId) -> Result<(), Error> {
        self.cmds.send(NodeCmd::RemoveNode(*id)).map_err(|_| Error::NodeTerminated)
    }
}

/// Dropping the [Node] cancels its internal tasks and all [Peer]s
impl Drop for Node {
    fn drop(&mut self) {
        self.task.abort();
    }
}
