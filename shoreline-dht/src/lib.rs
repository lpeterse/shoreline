mod common;
mod dht;
mod error;
mod net;
mod node;
mod peer;
mod util;

pub use self::common::{Id, Info, NodeId, NodeInfo, PeerId, PeerInfo, Version};
pub use self::dht::DHT;
pub use self::error::Error;
pub use self::net::InterfaceAddr;
pub use self::node::{Node, NodeStat};
pub use self::peer::{Peer, Status};

pub const SEEDS: &[&str] = &[
    "[2001:41d0:203:4cca:5::]:6881",  // dht.transmissionbt.com IPv6
    "[2a01:4f8:1c1a:1dba::1]:6881",   // dht.kats.network IPv6
    "dht.kats.network:6881"           // dht.kats.network IPv6
];
