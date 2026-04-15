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
    pub fn new(id: Id, name: String, addr: SocketAddrV6, peers: Peers, seeds: watch::Receiver<Vec<SocketAddrV6>>) -> Result<Arc<Self>, Error> {
        let (stat_, stat) = watch::channel(NodeStat::default());
        let cmds = mpsc::unbounded_channel();
        let cmdr = cmds.1;
        let cmds = cmds.0;
        let ctok = peers.ctok().child_token();
        let this = Arc::new(Self { id, name, addr, cmds, stat, token: ctok });
        Task::spawn(this.clone(), peers, seeds, stat_, cmdr)?;
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

    /// Suggest to connect to a [Peer] and eventually add it to the table
    ///
    /// A node is added to the table if it is responsive and either fills a
    /// gap in the routing table or is a better candidate than an existing node
    /// (e.g. has lower RTT).
    pub fn suggest(&self, info: &Info) -> Result<(), Error> {
        self.cmds.send(Command::Suggest(info.clone())).map_err(|_| Error::NodeTerminated)
    }

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
