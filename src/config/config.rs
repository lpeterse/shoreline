use serde::{Deserialize, Serialize};

use crate::identity::KeyPair;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(rename = "identity")]
    pub identity: IdentityConfig,
    #[serde(rename = "dht")]
    pub dht: DhtConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IdentityConfig {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "keypair")]
    pub keypair: KeyPair,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DhtConfig {
    #[serde(rename = "enabled")]
    pub enabled: bool,
    #[serde(rename = "port")]
    pub port: u16,
    #[serde(rename = "bootstrap_nodes")]
    pub bootstrap_nodes: Vec<String>,
}

impl Default for IdentityConfig {
    fn default() -> Self {
        Self { name: "Unknown Identity".to_string(), keypair: KeyPair::new() }
    }
}

impl Default for DhtConfig {
    fn default() -> Self {
        Self { enabled: true, port: 6881, bootstrap_nodes: vec![ "dht.kats.network:6881".to_string()] }
    }
}
