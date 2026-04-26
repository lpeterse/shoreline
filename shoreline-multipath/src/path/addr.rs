use std::net::SocketAddrV6;
use std::os::fd::AsRawFd;
use tokio::net::UdpSocket;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PathAddr {
    local: SocketAddrV6,
    remote: SocketAddrV6,
}

impl PathAddr {
    pub fn new(local: SocketAddrV6, remote: SocketAddrV6) -> Option<Self> {
        let lgua = Self::is_gua(local.ip());
        let rgua = Self::is_gua(remote.ip());
        let lula = Self::is_ula(local.ip());
        let rula = Self::is_ula(remote.ip());
        let llla = Self::is_lla(local.ip());
        let rlla = Self::is_lla(remote.ip());
        check(lgua || lula || llla)?;
        check(rgua || rula || rlla)?;
        check(lgua == rgua)?;
        check(lula == rula)?;
        check(llla == rlla)?;
        Some(Self { local, remote: SocketAddrV6::new(*remote.ip(), remote.port(), 0, local.scope_id()) })
    }

    pub fn iface(&self) -> u32 {
        self.local.scope_id()
    }

    pub fn local(&self) -> &SocketAddrV6 {
        &self.local
    }

    pub fn remote(&self) -> &SocketAddrV6 {
        &self.remote
    }

    pub fn connect(&self) -> Result<UdpSocket, std::io::Error> {
        use socket2::{Domain, Protocol, Socket, Type};
        let socket = Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::UDP))?;

        if cfg!(target_os = "macos") {
            let fd = socket.as_raw_fd();
            let option: i32 = 1;
            let res = unsafe {
                libc::setsockopt(
                    fd,
                    libc::SOL_SOCKET,
                    0x1104, // SO_RECV_ANYIF on macOS
                    &option as *const _ as *const libc::c_void,
                    std::mem::size_of::<i32>() as libc::socklen_t,
                )
            };

            if res == -1 {
                return Err(std::io::Error::last_os_error());
            }
        }

        socket.set_only_v6(true)?;
        socket.set_reuse_address(true)?;
        socket.set_reuse_port(true)?;
        socket.bind(&self.local.into())?;
        socket.connect(&self.remote.into())?;
        socket.set_nonblocking(true)?;
        Ok(UdpSocket::from_std(socket.into())?)
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
    if b { Some(()) } else { None }
}
