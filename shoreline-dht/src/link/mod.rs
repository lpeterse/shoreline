mod cmd;
mod stat;
mod status;
mod task;
mod trxs;

pub use self::status::Status;

use crate::common::Id;
use crate::common::Infos;
use crate::error::Error;
use crate::link::cmd::{CmdFindNode, CmdPing, Command};
use crate::link::stat::Stat;
use crate::link::task::Task;
use crate::{Node, Peer};
use std::net::SocketAddrV6;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
pub struct Link {
    node: Arc<Node>,
    peer: Arc<Peer>,
    addr: SocketAddrV6,
    cmds: mpsc::UnboundedSender<Command>,
    stat: watch::Receiver<Stat>,
    token: CancellationToken,
}

impl Link {
    pub fn new(node: Arc<Node>, peer: Arc<Peer>, addr: SocketAddrV6) -> Arc<Self> {
        let (cmds, cmds_) = mpsc::unbounded_channel();
        let (stat_, stat) = watch::channel(Stat::new());
        let token = Task::spawn(node.clone(), peer.clone(), addr, cmds_, stat_);
        Arc::new(Self { node, peer, addr, token, cmds, stat })
    }

    pub fn node(&self) -> &Arc<Node> {
        &self.node
    }

    pub fn peer(&self) -> &Arc<Peer> {
        &self.peer
    }

    pub fn addr(&self) -> &SocketAddrV6 {
        &self.addr
    }

    pub fn stat(&self) -> &watch::Receiver<Stat> {
        &self.stat
    }

    pub fn token(&self) -> &CancellationToken {
        &self.token
    }

    pub async fn init(&self) -> Result<Status, Error> {
        let mut stat = self.stat.clone();
        while matches!(stat.borrow().status, Status::Init) {
            stat.changed().await.map_err(|_| Error::LinkTerminated)?;
        }
        Ok(stat.borrow().status)
    }

    pub async fn ping(&self) -> Result<(), Error> {
        let (trx, rx) = CmdPing::new();
        self.cmds.send(trx.into()).map_err(|_| Error::LinkTerminated)?;
        Ok(rx.await.map_err(|_| Error::LinkTerminated)??)
    }

    pub async fn find_node(&self, id: &Id) -> Result<Infos, Error> {
        let (trx, rx) = CmdFindNode::new(id.clone());
        self.cmds.send(trx.into()).map_err(|_| Error::LinkTerminated)?;
        Ok(rx.await.map_err(|_| Error::LinkTerminated)??)
    }
}
