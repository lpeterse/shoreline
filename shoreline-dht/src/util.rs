use std::net::SocketAddrV6;
use tokio::net::UdpSocket;
use tokio::time::{Duration, Interval};

pub fn check(b: bool) -> Option<()> {
    if b { Some(()) } else { None }
}

pub fn interval_skip(duration: Duration) -> Interval {
    let mut intvl = tokio::time::interval(duration);
    intvl.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    intvl
}

pub fn interval_skip_at(start: tokio::time::Instant, duration: Duration) -> Interval {
    let mut intvl = tokio::time::interval_at(start, duration);
    intvl.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    intvl
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
