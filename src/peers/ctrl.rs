use tokio::{runtime::Runtime, sync::watch};

use crate::config::{ConfigCtrl, ConfigState, PeerConfig};

#[derive(Debug, Clone)]
pub struct PeersCtrl {
    peers: watch::Receiver<Vec<PeerConfig>>,
}

impl PeersCtrl {
    pub fn new(rt: &Runtime, config: ConfigCtrl) -> Self {
        let cfg = config.subscribe();
        let (tx, rx) = watch::channel(vec![]);
        rt.spawn(async move {
            let mut cfg = cfg;
            while cfg.changed().await.is_ok() {
                let peers = match { cfg.borrow().clone() } {
                    ConfigState::Result(Ok(Some(config))) => config.peers.list,
                    _ => vec![],
                };
                let _ = tx.send(peers);
            }
        });
        Self { peers: rx }
    }

    pub fn peers(&self) -> Vec<PeerConfig> {
        self.peers.borrow().clone()
    }
}
