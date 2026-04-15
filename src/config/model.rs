use serde::{Deserialize, Serialize};

use crate::{
    identity::KeyPair,
    model::{HostAddress, PublicKey},
};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(rename = "identity")]
    pub identity: IdentityConfig,
    #[serde(rename = "peers")]
    pub peers: PeersConfig,
    #[serde(rename = "dht")]
    pub dht: DhtConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IdentityConfig {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "pubkey")]
    pub pubkey: PublicKey,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeersConfig {
    #[serde(rename = "port")]
    pub port: u16,
    #[serde(rename = "list")]
    pub list: Vec<PeerConfig>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeerConfig {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "pubkey")]
    pub pubkey: PublicKey,
    #[serde(rename = "addresses")]
    pub addresses: Vec<HostAddress>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DhtConfig {
    #[serde(rename = "enabled")]
    pub enabled: bool,
    #[serde(rename = "port")]
    pub port: u16,
    #[serde(rename = "seeds")]
    pub seeds: Vec<HostAddress>,
}

impl Default for IdentityConfig {
    fn default() -> Self {
        Self { name: "Unknown Identity".to_string(), pubkey: PublicKey::random() }
    }
}

impl Default for DhtConfig {
    fn default() -> Self {
        Self { enabled: true, port: 6881, seeds: vec!["dht.kats.network:6881".parse().unwrap()] }
    }
}

impl Default for PeersConfig {
    fn default() -> Self {
        Self {
            port: 6882,
            list: vec![
                PeerConfig {
                    name: "Franz Nord".to_string(),
                    pubkey: PublicKey::random(),
                    addresses: vec!["franz.kats.network:6882".parse().unwrap()],
                },
            ],
        }
    }
}
