use std::net::SocketAddrV6;
use tokio::net::UdpSocket;

pub fn check(b: bool) -> Option<()> {
    if b { Some(()) } else { None }
}

pub fn socket() -> UdpSocket {
    use socket2::{Domain, Protocol, Socket, Type};
    let socket = Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::UDP)).unwrap();
    let _ = socket.set_nonblocking(true);
    UdpSocket::from_std(socket.into()).unwrap()
}

pub fn socket_bound(bind: SocketAddrV6) -> Result<UdpSocket, std::io::Error> {
    use socket2::{Domain, Protocol, Socket, Type};
    let socket = Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_only_v6(true)?;
    socket.set_reuse_address(true)?;
    socket.set_reuse_port(true)?;
    socket.bind(&bind.into())?;
    socket.set_nonblocking(true)?;
    Ok(UdpSocket::from_std(socket.into())?)
}

pub fn socket_connected(bind: &SocketAddrV6, conn: &SocketAddrV6) -> Result<UdpSocket, std::io::Error> {
    use socket2::{Domain, Protocol, Socket, Type};
    let socket = Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_only_v6(true)?;
    socket.set_reuse_address(true)?;
    socket.set_reuse_port(true)?;
    socket.bind(&(*bind).into())?;
    socket.connect(&(*conn).into())?;
    socket.set_nonblocking(true)?;
    Ok(UdpSocket::from_std(socket.into())?)
}
