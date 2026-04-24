use crate::util::interval_skip;
use ipnetwork::Ipv6Network;
use std::net::{IpAddr, Ipv6Addr};
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NetworkInterface {
    pub index: u32,
    pub name: String,
    pub description: String,
    pub addrs: Vec<Ipv6Addr>,
}

#[derive(Debug, Clone)]
pub struct Netwatch {
    #[allow(dead_code)]
    task: Arc<NetwatchTask>,
    list: watch::Receiver<Vec<NetworkInterface>>,
}

impl Netwatch {
    const INTERVAL: Duration = Duration::from_secs(10);

    pub fn new(rt: &tokio::runtime::Runtime) -> Self {
        let (list_, list) = watch::channel(Vec::new());
        let task = rt.spawn(async move {
            let mut interval = interval_skip(Self::INTERVAL);
            let list = list_;
            loop {
                tokio::select! {
                    _ = interval.tick() => {}
                    _ = list.closed() => {
                        break;
                    }
                }
                let mut new_interfaces = Vec::new();
                for interface in pnet_datalink::interfaces().into_iter().filter(Self::is_valid_interface) {
                    let is_loopback = interface.is_loopback();
                    let is_point_to_point = interface.is_point_to_point();
                    let mut new_interface = NetworkInterface {
                        index: interface.index,
                        name: interface.name,
                        description: interface.description,
                        addrs: vec![],
                    };
                    let mut networks = vec![];
                    // For each interface prefer the first address of a kind:
                    // These are considered the stable addresses. For GUA, take the first of a network in
                    // case the interface has mulitple addresses in different GUA networks.
                    let mut ula: bool = false;
                    let mut lla: bool = false;
                    for ip in interface.ips.into_iter() {
                        match ip {
                            ipnetwork::IpNetwork::V6(v6) => {
                                let mut add = false;
                                let ip = v6.ip();
                                let sn = |n: &Ipv6Network| n.network() == v6.network();
                                if Netwatch::is_gua(&ip) && !networks.iter().any(sn) {
                                    add = true;
                                }
                                if Netwatch::is_ula(&ip) && v6.prefix() == 64 && !ula {
                                    ula = true;
                                    add = true;
                                }
                                if Netwatch::is_lla(&ip) && v6.prefix() == 64 && !lla  {
                                    if !is_loopback && !is_point_to_point {
                                        lla = true;
                                        add = true;
                                    }
                                }
                                if add {
                                    networks.push(v6);
                                }
                            }
                            _ => {}
                        }
                    }
                    if !networks.is_empty() {
                        new_interface.addrs = networks.into_iter().map(|n| n.ip()).collect();
                        new_interface.addrs.sort();
                        new_interfaces.push(new_interface);
                    }
                }

                let b = list.borrow();
                if b.deref() != &new_interfaces {
                    drop(b);
                    let _ = list.send(new_interfaces);
                }
            }
        });
        Self { list, task: Arc::new(NetwatchTask(task)) }
    }

    pub async fn changed(&mut self) {
        self.list.changed().await.unwrap();
    }

    pub fn list(&self) -> Vec<NetworkInterface> {
        self.list.borrow().clone()
    }

    pub fn ips(&self) -> Vec<IpAddr> {
        let mut ips = vec![];
        for interface in self.list.borrow().iter() {
            for addr in &interface.addrs {
                ips.push(IpAddr::V6(*addr));
            }
        }
        ips.sort();
        ips
    }

    /// Check if the address is a global unicast address
    pub fn is_gua(ip: &std::net::Ipv6Addr) -> bool {
        !(ip.is_loopback()
            || ip.is_unspecified()
            || ip.is_multicast()
            || ip.is_unique_local()
            || ip.is_unicast_link_local())
    }

    /// Check if the address is a unique local address
    pub fn is_ula(ip: &std::net::Ipv6Addr) -> bool {
        ip.is_unique_local()
    }

    /// Check if the address is a link local address
    pub fn is_lla(ip: &std::net::Ipv6Addr) -> bool {
        ip.is_unicast_link_local()
    }

    fn is_valid_interface(interface: &pnet_datalink::NetworkInterface) -> bool {
        if cfg!(target_os = "macos") {
            if interface.name.starts_with("anpi") {
                return false;
            }
            // if interface.name.starts_with("awdl") {
            //     return false;
            // }
            if interface.name.starts_with("llw") {
                return false;
            }
            if interface.name.starts_with("gif") {
                return false;
            }
            if interface.name.starts_with("bridge") {
                return false;
            }
        }
        true
    }
}

#[derive(Debug)]
struct NetwatchTask(tokio::task::JoinHandle<()>);

impl Drop for NetwatchTask {
    fn drop(&mut self) {
        self.0.abort();
    }
}
