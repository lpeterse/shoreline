use crate::Error;

use super::super::{Peer, Id, Info};
use super::super::common::Infos;
use std::net::SocketAddrV6;
use std::sync::Arc;
use tokio::sync::oneshot;

#[derive(Debug)]
pub enum NodeCmd {
    Seed(SocketAddrV6),
    GetNode(Info, oneshot::Sender<Result<Arc<Peer>, Error>>),
    GetNodes(oneshot::Sender<Vec<Arc<Peer>>>),
    FindNode(Id, oneshot::Sender<Infos>),
    RemoveNode(Id),
    SuggestNode(Info),
}
