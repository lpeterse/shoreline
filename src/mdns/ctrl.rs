use crate::config::ConfigState;
use crate::{config::ConfigCtrl, model::PublicKey};
use mdns_sd::{ScopedIp, ScopedIpV6, ServiceDaemon, ServiceEvent, ServiceInfo};
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use std::time::Duration;
use std::{collections::BTreeMap, str::FromStr};
use tokio::{runtime::Runtime, select, sync::watch};

#[derive(Debug, Clone)]
pub struct MdnsCtrl {
    rx: watch::Receiver<BTreeMap<PublicKey, MdnsEntry>>,
}

impl MdnsCtrl {
    pub fn new(rt: &Runtime, config: ConfigCtrl) -> Self {
        let (a, b) = watch::channel(BTreeMap::new());
        rt.spawn(MdnsCtrlTask::new(config, a).run());
        Self { rx: b }
    }

    pub fn entries(&self) -> Vec<MdnsEntry> {
        self.rx.borrow().values().cloned().collect()
    }
}

#[derive(Debug, Clone)]
pub struct MdnsEntry {
    pub pubkey: PublicKey,
    pub fullname: String,
    pub displayname: String,
    pub port: u16,
    pub addrs: Vec<ScopedIpV6>,
}

struct MdnsCtrlTask {
    config: watch::Receiver<ConfigState>,
    tx: watch::Sender<BTreeMap<PublicKey, MdnsEntry>>,
    id: Option<(PublicKey, String, u16)>,
    token: CancellationToken,
    child: CancellationToken
}

impl MdnsCtrlTask {
    pub fn new(config: ConfigCtrl, tx: watch::Sender<BTreeMap<PublicKey, MdnsEntry>>) -> Self {
        let token = CancellationToken::new();
        let child = token.child_token();
        Self { config: config.subscribe(), tx, id: None, token, child }
    }

    pub async fn run(mut self) {
        let token = self.token.clone();
        select! {
            _ = token.cancelled() => {}
            _ = self.run_() => {}
        }
    }

    pub async fn run_(&mut self) {
        loop {
            if self.check_config() {
                self.restart();
            }

            select! {
                _ = self.child.cancelled() => {
                    sleep(Duration::from_secs(10)).await;
                }
                _ = self.config.changed() => {}
            }
        }
    }

    fn check_config(&mut self) -> bool {
        let config = self.config.borrow().clone();
        if let ConfigState::Result(Ok(Some(config))) = config {
            let id = Some((config.identity.pubkey.clone(), config.identity.name.clone(), config.peers.port));
            if self.id != id {
                self.child.cancel();
                self.id = id;
                return true;
            } else {
                return false;
            }
        } else {
            self.id = None;
            return false;
        }
    }

    fn restart(&mut self) {
        if let Some((pubkey, name, port)) = self.id.clone() {
            self.child = self.token.child_token();
            tokio::spawn(Self::run_mdns(pubkey, name, port, self.tx.clone(), self.child.clone()));
        }
    }

    async fn run_mdns(pubkey: PublicKey, name: String, port: u16, tx: watch::Sender<BTreeMap<PublicKey, MdnsEntry>>, token: CancellationToken) {
        log::info!("Starting mDNS daemon with pubkey {} and port {}", pubkey, port);
        select! {
            r = Self::run_mdns_error(pubkey, name, port, tx) => {
                token.cancel();
                if let Err(e) = r {
                    log::error!("mDNS daemon error: {}", e);
                }
            },
            _ = token.cancelled() => {
                log::info!("mDNS daemon cancelled");
                return;
            }
        }
    }

    async fn run_mdns_error(pubkey: PublicKey, displayname: String, port: u16, tx: watch::Sender<BTreeMap<PublicKey, MdnsEntry>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let daemon = ServiceDaemon::new()?;

        // Register the service with the daemon
        let service = format!("_{}._udp.local.", env!("CARGO_PKG_NAME"));
        let name = pubkey.to_string().chars().take(16).collect::<String>();
        let host = format!("{}.local.", name);
        let port = port;
        let prop = [("pubkey", pubkey.to_string()), ("displayname", displayname.clone())];
        let info = ServiceInfo::new(&service, &name, &host, (), port, &prop[..])?;
        let mut info = info.enable_addr_auto();
        info.set_link_local_only(true);
        daemon.register(info)?;

        // Browse for the service on the local network
        let receiver = daemon.browse(&service)?;
        loop {
            match receiver.recv_async().await? { 
                ServiceEvent::ServiceResolved(resolved) => {
                    let pubkey = resolved.txt_properties.get("pubkey");
                    let pubkey = pubkey.and_then(|s| PublicKey::from_str(s.val_str()).ok());
                    let displayname = resolved.txt_properties.get("displayname");
                    let displayname = displayname.and_then(|s| Some(s.val_str().to_string())).unwrap_or_default();
                    if let Some(pubkey) = pubkey {
                        let f = |ip: ScopedIp| if let ScopedIp::V6(ip) = ip { Some(ip) } else { None };
                        let g = |ip: ScopedIpV6| if ip.addr().to_string() != "fe80::1" { Some(ip) } else { None };
                        let addrs = resolved.addresses.into_iter().filter_map(f).filter_map(g).collect();
                        let service = MdnsEntry { pubkey, displayname, port: resolved.port, addrs, fullname: resolved.fullname.clone() };
                        tx.send_modify(|services| { services.insert(service.pubkey.clone(), service); });
                    }
                },
                ServiceEvent::ServiceRemoved(_, fullname) => {
                    let mut removed = None;
                    tx.send_modify(|services| {
                        if let Some((key, _)) = services.iter().find(|(_, s)| s.fullname == fullname) {
                            removed = Some(key.clone());
                        }
                        if let Some(key) = &removed {
                            services.remove(key);
                        }
                    });
                },
                _ => {}
            }
        }
    }
}
