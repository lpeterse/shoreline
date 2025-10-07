use std::{net::SocketAddrV6, sync::Arc};
use tokio::{net::UdpSocket, sync::SetOnce};
use tokio::time::{Duration, Instant, Sleep, sleep_until};

#[derive(Debug)]
pub struct Promise<T>(Arc<SetOnce<T>>);

impl <T> Promise<T> {
    pub fn get(&self) -> Option<&T> {
        self.0.get()
    }

    pub async fn wait(&self) -> &T {
        self.0.wait().await
    }
}

impl <T> From<Arc<SetOnce<T>>> for Promise<T> {
    fn from(v: Arc<SetOnce<T>>) -> Self {
        Self(v)
    }
}

pub fn check(b: bool) -> Option<()> {
    if b {
        Some(())
    } else {
        None
    }
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

pub struct Backoff {
    max: Duration,
    exp: u32,
    timeout: Instant,
}

impl Backoff {
    pub fn new(max: Duration) -> Self {
        Self { max, exp: 0, timeout: Instant::now() }
    }

    pub fn reset(&mut self) {
        self.exp = 0;
        self.timeout = Instant::now();
    }

    pub fn tick(&mut self) -> Sleep {
        let t = self.timeout;
        if self.timeout < Instant::now() {
            let duration = Duration::from_secs(1 << self.exp).min(self.max);
            self.timeout = Instant::now() + duration;
            self.exp += 1;
        }
        sleep_until(t)
    }
}
