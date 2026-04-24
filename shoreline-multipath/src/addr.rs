use std::net::SocketAddrV6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SocketAddrPair {
    pub local: SocketAddrV6,
    pub remote: SocketAddrV6,
}

impl SocketAddrPair {
    pub fn is_valid(&self) -> bool {
        if self.local.scope_id() != self.remote.scope_id() {
            return false;
        }
        if Self::is_gua(self.local.ip()) != Self::is_gua(self.remote.ip()) {
            return false;
        }
        if Self::is_ula(self.local.ip()) != Self::is_ula(self.remote.ip()) {
            return false;
        }
        if Self::is_lla(self.local.ip()) != Self::is_lla(self.remote.ip()) {
            return false;
        }
        true
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
