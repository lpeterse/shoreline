use super::super::common::{Id, Infos};
use super::super::Error;
use tokio::sync::oneshot;

#[derive(Debug)]
pub enum PeerCmd {
    Ping(CmdPing),
    FindNode(CmdFindNode),
}

impl PeerCmd {
    pub fn reject(self, e: Error) {
        match self {
            PeerCmd::Ping(t) => {
                let _ = t.response.send(Err(e));
            }
            PeerCmd::FindNode(t) => {
                let _ = t.response.send(Err(e));
            }
        }
    }
}

#[derive(Debug)]
pub struct CmdPing {
    pub response: oneshot::Sender<Result<(), Error>>,
}

impl CmdPing {
    pub fn new() -> (Self, oneshot::Receiver<Result<(), Error>>) {
        let (tx, rx) = oneshot::channel();
        (Self { response: tx }, rx)
    }
}

#[derive(Debug)]
pub struct CmdFindNode {
    pub target: Id,
    pub response: oneshot::Sender<Result<Infos, Error>>,
}

impl CmdFindNode {
    pub fn new(target: Id) -> (Self, oneshot::Receiver<Result<Infos, Error>>) {
        let (tx, rx) = oneshot::channel();
        (Self { target, response: tx }, rx)
    }
}

impl Into<PeerCmd> for CmdPing {
    fn into(self) -> PeerCmd {
        PeerCmd::Ping(self)
    }
}

impl Into<PeerCmd> for CmdFindNode {
    fn into(self) -> PeerCmd {
        PeerCmd::FindNode(self)
    }
}
