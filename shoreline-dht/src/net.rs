use ipnetwork::Ipv6Network;
use std::sync::Arc;
use tokio::sync::watch;
use tokio::time::{Duration, interval};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InterfaceAddr {
    pub interface: String,
    pub addr: std::net::Ipv6Addr,
    pub prefix: u8,
}

impl std::fmt::Display for InterfaceAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let prefix = self.prefix.min(128);
        let network = Ipv6Network::new(self.addr, prefix).unwrap().network();
        let network = Ipv6Network::new(network, prefix).unwrap();
        write!(f, "{}: {}", self.interface, network)
    }
}

#[derive(Debug, Clone)]
pub struct Netwatch {
    #[allow(dead_code)]
    task: Arc<NetwatchTask>,
    list: watch::Receiver<Vec<InterfaceAddr>>,
}

impl Netwatch {
    const INTERVAL: Duration = Duration::from_secs(10);

    pub fn new() -> Self {
        let (list_, list) = watch::channel(vec![]);
        let task = tokio::task::spawn(async move {
            let mut interval = interval(Self::INTERVAL);
            let list = list_;
            loop {
                tokio::select! {
                    _ = interval.tick() => {}
                    _ = list.closed() => {
                        break;
                    }
                }
                let mut addresses = vec![];
                for interface in pnet_datalink::interfaces().iter().filter(|i| i.is_up() && !i.is_loopback()) {
                    // For each interface only consider the first GUA and ULA address.
                    // These are considered the stable addresses for the interface.
                    let mut gua: bool = false;
                    let mut ula: bool = false;
                    for ip in interface.ips.iter() {
                        match ip {
                            ipnetwork::IpNetwork::V6(v6) => {
                                let mut add = false;
                                let ip = v6.ip();
                                if !gua && Netwatch::is_gua(&ip) {
                                    gua = true;
                                    add = true;
                                }
                                if !ula && Netwatch::is_ula(&ip) && v6.prefix() == 64 {
                                    ula = true;
                                    add = true;
                                }
                                if add {
                                    let interface = interface.name.clone();
                                    let addr = v6.ip();
                                    let prefix = v6.prefix();
                                    addresses.push(InterfaceAddr { interface, addr, prefix });
                                }
                            }
                            _ => {}
                        }
                    }
                }
                let b = list.borrow();
                let v: &Vec<InterfaceAddr> = b.as_ref();
                if v != &addresses {
                    drop(b);
                    let _ = list.send(addresses);
                }
            }
        });
        Self { list, task: Arc::new(NetwatchTask(task)) }
    }

    pub async fn changed(&mut self) {
        self.list.changed().await.unwrap();
    }

    pub fn list(&self) -> Vec<InterfaceAddr> {
        self.list.borrow().clone()
    }

    /// Check if the address is a global unicast address
    fn is_gua(ip: &std::net::Ipv6Addr) -> bool {
        !(ip.is_loopback()
            || ip.is_unspecified()
            || ip.is_multicast()
            || ip.is_unique_local()
            || ip.is_unicast_link_local())
    }

    /// Check if the address is a unique local address
    fn is_ula(ip: &std::net::Ipv6Addr) -> bool {
        ip.is_unique_local()
    }
}

#[derive(Debug)]
struct NetwatchTask(tokio::task::JoinHandle<()>);

impl Drop for NetwatchTask {
    fn drop(&mut self) {
        self.0.abort();
    }
}
