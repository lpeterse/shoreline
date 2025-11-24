use super::super::{Id, Info};
use super::super::common::Infos;
use tokio::sync::oneshot;

#[derive(Debug)]
pub enum Command {
    Suggest(Info),
    FindNode(Id, oneshot::Sender<Infos>),
}
