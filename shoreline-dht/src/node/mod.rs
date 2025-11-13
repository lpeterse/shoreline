mod cmd;
mod stat;
mod task;

use self::task::Task;
use super::{Error, Version};
use crate::{Id, Info};
use crate::Peers;
use std::net::SocketAddrV6;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::{oneshot, watch};
use tokio_util::sync::CancellationToken;

pub use self::cmd::Command;
pub use self::stat::NodeStat;

/// A client for the Mainline DHT network
#[derive(Debug)]
pub struct Node {
    id: Id,
    name: String,
    addr: SocketAddrV6,
    cmds: mpsc::UnboundedSender<Command>,
    stat: watch::Receiver<NodeStat>,
    token: CancellationToken,
}

impl Node {
    /// Create a new [Node] node with the given [Info]
    pub fn new(id: Id, name: String, addr: SocketAddrV6, peers: Peers) -> Result<Arc<Self>, Error> {
        let (stat_, stat) = watch::channel(NodeStat::default());
        let cmds = mpsc::unbounded_channel();
        let cmdr = cmds.1;
        let cmds = cmds.0;
        let ctok = peers.ctok().child_token();
        let this = Arc::new(Self { id, name, addr, cmds, stat, token: ctok });
        Task::spawn(this.clone(), peers, stat_, cmdr)?;
        Ok(this)
    }

    /// Get this node's [Id]
    pub fn id(&self) -> &Id {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get this node's [SocketAddrV6]
    pub fn addr(&self) -> &SocketAddrV6 {
        &self.addr
    }

    /// Get this node's [Version] (a.k.a. client identifier)
    pub fn version(&self) -> Version {
        Version::SELF
    }

    /// Get this node's current [NodeStat]
    pub fn stat(&self) -> NodeStat {
        self.stat.borrow().clone()
    }

    ///
    pub fn seed(&self, addr: &SocketAddrV6){
        let _ = self.cmds.send(Command::Seed((*addr).into()));
    }

    /// Suggest to connect to a [Peer] and eventually add it to the table
    ///
    /// A node is added to the table if it is responsive and either fills a
    /// gap in the routing table or is a better candidate than an existing node
    /// (e.g. has lower RTT).
    pub fn suggest(&self, info: &Info) -> Result<(), Error> {
        self.cmds.send(Command::Suggest(info.clone())).map_err(|_| Error::NodeTerminated)
    }

    // /// Search the DHT for a [Id] using A* search
    // pub async fn search(&self, id: &Id) -> Result<Info, Error> {
    //     type Q = BTreeMap<Id, Info>;
    //     type V = BTreeSet<Id>;
    //     let f = |n: Arc<PeerConn>| (n.id().xor(id), Info::new(*n.id(), *n.address()));
    //     let m = self.peers().borrow();
    //     let mut v: V = m.keys().map(|d| d.xor(self.id())).collect();
    //     let mut q: Q = m.values().filter_map(Weak::upgrade).map(f).collect();

    //     while let Some((_, info)) = q.pop_first() {
    //         if &info.id == id {
    //             return Ok(info);
    //         }
    //         let n = self.peer(&info).await?;
    //         if let Ok(ns) = n.find_node(id).await {
    //             for info in ns.iter() {
    //                 if &info.id == id {
    //                     return Ok(*info);
    //                 }
    //                 if &info.id != self.id() && v.insert(info.id) {
    //                     q.insert(info.id.xor(id), *info);
    //                 }
    //             }
    //         }
    //     }

    //     Err(format!("Asked {} peers: not found", v.len()).into())
    // }

    // /// Lookup or create a [Peer] by [Info]
    // ///
    // /// Either returns an existing instance or creates a new one using the given
    // /// [Id] and [SocketAddrV6]. The node might be added to the routing
    // /// table if considered appropriate.
    // pub async fn peer(&self, info: &Info) -> Result<Arc<PeerConn>, Error> {
    //     let (tx, rx) = oneshot::channel();
    //     self.cmds.send(NodeCmd::GetNode(info.clone(), tx)).map_err(|_| Error::NodeTerminated)?;
    //     Ok(rx.await.map_err(|_| Error::NodeTerminated)??)
    // }

    // /// Get a list of known [Peer]s
    // ///
    // /// This includes peers not necessarily in the routing table. Such peers might
    // /// exist because e.g. a search process has a strong handle on it.
    // pub fn peers(&self) -> &watch::Receiver<BTreeMap<Id, Weak<PeerConn>>> {
    //     &self.peers
    // }

    /// Find [Info]s close to the given [Id]
    ///
    /// Currently returns a list of up to 8 peers closest to the given id.
    /// This does not perform any network operations, but is just a lookup in the routing table.
    pub async fn find(&self, id: &Id) -> Result<Vec<Info>, Error> {
        let (tx, rx) = oneshot::channel();
        self.cmds.send(Command::FindNode(*id, tx)).map_err(|_| Error::NodeTerminated)?;
        rx.await.map(Into::into).map_err(|_| Error::NodeTerminated)
    }

    pub fn token(&self) -> &CancellationToken {
        &self.token
    }
}
