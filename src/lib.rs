pub mod config;
pub mod util;
pub mod mmdb;
pub mod app;

pub const SEEDS: &[&str] = &[
    "[2001:41d0:203:4cca:5::]:6881", // dht.transmissionbt.com IPv6
    "[2a01:4f8:1c1a:1dba::1]:6881",  // dht.kats.network IPv6
    "dht.kats.network:6881",         // dht.kats.network IPv6
    "[fd23:80c1:7b27::1]:6881"
];