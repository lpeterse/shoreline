use super::super::common::{Id, Infos};
use super::super::Error;
use tokio::sync::oneshot;

#[derive(Debug)]
pub enum Command {
    Ping(CmdPing),
    FindNode(CmdFindNode),
}

impl Command {
    pub fn reject(self, e: Error) {
        match self {
            Command::Ping(t) => {
                let _ = t.response.send(Err(e));
            }
            Command::FindNode(t) => {
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

impl Into<Command> for CmdPing {
    fn into(self) -> Command {
        Command::Ping(self)
    }
}

impl Into<Command> for CmdFindNode {
    fn into(self) -> Command {
        Command::FindNode(self)
    }
}
