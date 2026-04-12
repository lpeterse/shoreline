use shoreline_dht::DHT;
use std::{ops::Deref, sync::Arc};
use tokio::{runtime::Runtime, sync::watch};

use crate::config::{ConfigCtrl, ConfigState};

#[derive(Debug, Clone)]
pub struct DhtCtrl {
    dht: watch::Receiver<Option<Arc<DHT>>>,
}

impl DhtCtrl {
    pub fn new(rt: &Runtime, config: ConfigCtrl) -> Self {
        let c = config.subscribe();
        let (a, b) = watch::channel(None);
        rt.spawn(DhtCtrlTask::new(a, c).run());
        Self { dht: b }
    }

    pub fn dht(&self) -> Option<Arc<DHT>> {
        self.dht.borrow().clone()
    }
}

struct DhtCtrlTask {
    cfg: watch::Receiver<ConfigState>,
    dht: watch::Sender<Option<Arc<DHT>>>,
    current_id: shoreline_dht::Id,
    current_port: u16,
    current_seeds: watch::Sender<Vec<std::net::SocketAddrV6>>,
}

impl DhtCtrlTask {
    pub fn new(dht: watch::Sender<Option<Arc<DHT>>>, cfg: watch::Receiver<ConfigState>) -> Self {
        Self {
            cfg,
            dht,
            current_id: shoreline_dht::Id::random(),
            current_port: 0,
            current_seeds: watch::channel(vec![]).0,
        }
    }

    pub async fn run(mut self) {
        while self.cfg.changed().await.is_ok() {
            match { self.cfg.borrow().clone() } {
                ConfigState::Loading => {
                    // Do nothing, wait for the next state
                }
                ConfigState::Result(Ok(Some(config))) => {
                    if config.dht.enabled {
                        let id = config.identity.keypair.pubkey_as_dht_id();
                        let port = config.dht.port;
                        if self.dht.borrow().is_none() || self.current_id != id || self.current_port != port {
                            self.current_id = id;
                            self.current_port = port;
                            let _ = self.dht.send(None);
                            let dht = DHT::new(self.current_id, self.current_port, self.current_seeds.subscribe());
                            let _ = self.dht.send(Some(Arc::new(dht)));
                        }
                        let mut seeds = vec![];
                        for seed in config.dht.seeds {
                            let _ = seed.resolve().await.map(|a| seeds.push(a)).ok();
                        }
                        if self.current_seeds.borrow().deref() != &seeds {
                            let _ = self.current_seeds.send(seeds);
                        }
                    } else {
                        let _ = self.dht.send(None);
                    }
                }
                ConfigState::Result(Ok(None)) | ConfigState::Result(Err(_)) => {
                    let _ = self.dht.send(None);
                }
            }
        }
    }
}
