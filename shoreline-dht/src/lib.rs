mod common;
mod link;
mod dht;
mod error;
mod net;
mod node;
mod nodes;
mod peer;
mod peers;
mod util;
mod constants;

pub use self::common::{Id, Info, Version};
pub use self::link::{Link, Status};
pub use self::dht::DHT;
pub use self::error::Error;
pub use self::node::{Node, NodeStat};
pub use self::nodes::Nodes;
pub use self::peer::Peer;
pub use self::peers::Peers;

pub const SEEDS: &[&str] = &[
    "[2001:41d0:203:4cca:5::]:6881", // dht.transmissionbt.com IPv6
    "[2a01:4f8:1c1a:1dba::1]:6881",  // dht.kats.network IPv6
    "dht.kats.network:6881",         // dht.kats.network IPv6
];
