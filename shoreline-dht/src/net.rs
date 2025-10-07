use std::sync::Arc;
use ipnetwork::Ipv6Network;
use tokio::sync::watch;

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
    task: Arc<AddressTask>,
    list: watch::Receiver<Vec<InterfaceAddr>>,
}

impl Netwatch {
    pub fn new() -> Self {
        let (list_, list) = watch::channel(vec![]);
        let task = tokio::task::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
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
                                    addresses.push(InterfaceAddr { interface: interface.name.clone(), addr: v6.ip(), prefix: v6.prefix() });
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
        Self { list: list, task: Arc::new(AddressTask(task)) }
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
struct AddressTask(tokio::task::JoinHandle<()>);

impl Drop for AddressTask {
    fn drop(&mut self) {
        self.0.abort();
    }
}
