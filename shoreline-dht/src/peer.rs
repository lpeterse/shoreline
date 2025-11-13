mod cmd;
mod stat;
mod status;
mod task;
mod trxs;

use self::cmd::{CmdFindNode, CmdPing, PeerCmd};
use self::stat::PeerStat;
use self::task::PeerTask;
use super::common::Infos;
use super::node::NodeCmd;
use super::{Error, Id, Info, Version};
use crate::util::socket_connected;
use std::net::SocketAddrV6;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::Duration;

pub use self::status::Status;

#[derive(Debug)]
pub struct Peer {
    pinf: Info,
    pcmd: mpsc::UnboundedSender<PeerCmd>,
    ncmd: mpsc::WeakUnboundedSender<NodeCmd>,
    task: JoinHandle<()>,
    stat: watch::Receiver<PeerStat>,
}

impl Peer {
    pub fn new(pinf: Info, ninf: Info, ncmd: mpsc::WeakUnboundedSender<NodeCmd>) -> Result<Arc<Self>, Error> {
        let sock = socket_connected(&ninf.addr, &pinf.addr).map_err(Error::Socket)?;
        let (pcmd, pcmd_) = mpsc::unbounded_channel();
        let (stat, stat_) = watch::channel(PeerStat::default());
        let ncmd_ = ncmd.clone();
        let task = PeerTask::spawn(pinf, ninf, sock, pcmd_, ncmd_, stat);
        let self_ = Self { pinf, task, pcmd, ncmd, stat: stat_ };
        Ok(Arc::new(self_))
    }

    pub fn id(&self) -> &Id {
        &self.pinf.id
    }

    pub fn info(&self) -> &Info {
        &self.pinf
    }

    pub fn status(&self) -> Status {
        self.stat.borrow().status
    }

    pub fn rtt(&self) -> Option<Duration> {
        self.stat.borrow().rtt
    }

    pub fn address(&self) -> &SocketAddrV6 {
        &self.pinf.addr
    }

    pub fn version(&self) -> Option<Version> {
        self.stat.borrow().version
    }

    pub fn error(&self) -> Option<Arc<Error>> {
        self.stat.borrow().error.clone()
    }

    pub fn stat(&self) -> &watch::Receiver<PeerStat> {
        &self.stat
    }

    pub async fn init(&self) -> Result<Status, Error> {
        let mut stat = self.stat.clone();
        while matches!(stat.borrow().status, Status::Init) {
            stat.changed().await.map_err(|_| Error::PeerTerminated)?;
        }
        Ok(stat.borrow().status)
    }

    pub async fn ping(&self) -> Result<(), Error> {
        let (trx, rx) = CmdPing::new();
        self.pcmd.send(trx.into()).map_err(|_| Error::PeerTerminated)?;
        Ok(rx.await.map_err(|_| Error::PeerTerminated)??)
    }

    pub async fn find_node(&self, id: &Id) -> Result<Infos, Error> {
        let (trx, rx) = CmdFindNode::new(id.clone());
        self.pcmd.send(trx.into()).map_err(|_| Error::PeerTerminated)?;
        Ok(rx.await.map_err(|_| Error::PeerTerminated)??)
    }
}

impl Drop for Peer {
    fn drop(&mut self) {
        self.ncmd.upgrade().map(|ncmd| ncmd.send(NodeCmd::RemoveNode(self.pinf.id)));
        self.task.abort();
    }
}
