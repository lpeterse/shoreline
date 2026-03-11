use serde::{Deserialize, Serialize};

use crate::{identity::KeyPair, model::{HostAddress, PublicKey}};

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
    #[serde(rename = "keypair")]
    pub keypair: KeyPair,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeersConfig {
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
        Self { name: "Unknown Identity".to_string(), keypair: KeyPair::new() }
    }
}

impl Default for DhtConfig {
    fn default() -> Self {
        Self { enabled: true, port: 6881, seeds: vec![ "dht.kats.network:6881".parse().unwrap()] }
    }
}

impl Default for PeersConfig {
    fn default() -> Self {
        Self { list: vec![
            PeerConfig {
                name: "Franz Nord".to_string(),
                pubkey: PublicKey::random(),
                addresses: vec![ "franz.kats.network:6882".parse().unwrap() ],
            },
             PeerConfig {
                name: "Sophie Müller".to_string(),
                pubkey: PublicKey::random(),
                addresses: vec![ "sophie.kats.network:6883".parse().unwrap(), "foobar.kats.network:6883".parse().unwrap() ],
            },
            PeerConfig {
                name: "Hans Anders".to_string(),
                pubkey: PublicKey::random(),
                addresses: vec![ "hans.kats.network:6884".parse().unwrap() , "foobar.kats.network:6884".parse().unwrap() ],
            },
            PeerConfig {
                name: "Maria Garcia".to_string(),
                pubkey: PublicKey::random(),
                addresses: vec![ "maria.kats.network:6885".parse().unwrap(), "foobar.kats.network:6885".parse().unwrap() ],
            },
            PeerConfig {
                name: "Erika Schmidt".to_string(),
                pubkey: PublicKey::random(),
                addresses: vec![ "erika.kats.network:6886".parse().unwrap(), "foobar.kats.network:6887".parse().unwrap() ],
            },
        ] }
    }
}
