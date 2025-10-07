use super::Id;
use std::net::SocketAddrV6;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Info {
    pub id: Id,
    pub addr: SocketAddrV6,
}

impl Info {
    pub fn new(id: Id, addr: SocketAddrV6) -> Self {
        Self { id, addr }
    }
}
