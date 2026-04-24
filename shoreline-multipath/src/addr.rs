use std::net::SocketAddrV6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SocketAddrPair {
    pub local: SocketAddrV6,
    pub remote: SocketAddrV6,
}

impl SocketAddrPair {
    pub fn new(local: SocketAddrV6, mut remote: SocketAddrV6) -> Option<Self> {
        remote.set_scope_id(local.scope_id());
        check(Self::is_gua(local.ip()) || Self::is_ula(local.ip()) || Self::is_lla(local.ip()))?;
        check(Self::is_gua(remote.ip()) || Self::is_ula(remote.ip()) || Self::is_lla(remote.ip()))?;
        check(Self::is_gua(local.ip()) == Self::is_gua(remote.ip()))?;
        check(Self::is_ula(local.ip()) == Self::is_ula(remote.ip()))?;
        check(Self::is_lla(local.ip()) == Self::is_lla(remote.ip()))?;
        Some(Self { local, remote })
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

    /// Check if the address is a link local address
    fn is_lla(ip: &std::net::Ipv6Addr) -> bool {
        ip.is_unicast_link_local()
    }
}

fn check(b: bool) -> Option<()> {
    if b {
        Some(())
    } else {
        None
    }
}
