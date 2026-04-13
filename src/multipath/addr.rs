use std::net::SocketAddrV6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SocketAddrPair {
    pub local: SocketAddrV6,
    pub remote: SocketAddrV6,
}
