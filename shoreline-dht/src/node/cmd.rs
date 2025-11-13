use super::super::{Id, Info};
use super::super::common::Infos;
use std::net::SocketAddrV6;
use tokio::sync::oneshot;

#[derive(Debug)]
pub enum Command {
    Seed(SocketAddrV6),
    Suggest(Info),
    FindNode(Id, oneshot::Sender<Infos>),
}
