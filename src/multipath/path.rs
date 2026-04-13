use std::net::SocketAddrV6;
use tokio::net::UdpSocket;
use tokio::select;
use tokio::sync::{mpsc, watch};
use tokio::time::{sleep, interval};
use tokio::time::{Duration};
use tokio_util::sync::CancellationToken;
use super::stats::{PathStats};
use super::timings::{Timings, MsgTimings};
use super::addr::SocketAddrPair;

pub struct Path {
    addr: SocketAddrPair,
    stats: watch::Receiver<PathStats>,
    token: CancellationToken,
}

impl Path {
    pub fn new(addr: SocketAddrPair, token: CancellationToken) -> Self {
        let (stats_tx, stats_rx) = watch::channel(PathStats::new());
        let task = PathTask::new(addr, stats_tx);
        let _ = tokio::spawn(task.run(token.clone()));
        Path { addr, stats: stats_rx, token }
    }

    pub fn addr(&self) -> SocketAddrPair {
        self.addr
    }

    pub fn stats(&self) -> &watch::Receiver<PathStats> {
        &self.stats
    }
}

impl Drop for Path {
    fn drop(&mut self) {
        self.token.cancel();
    }
}

pub struct PathTask {
    addr: SocketAddrPair,
    timings: Timings,
    rbuf: Vec<u8>,
    upstream: mpsc::UnboundedSender<Vec<u8>>,
    socket: Result<UdpSocket, std::io::Error>,
    stats: watch::Sender<PathStats>,
}

impl PathTask {
    pub fn new(addr: SocketAddrPair, stats: watch::Sender<PathStats>) -> Self {
        let socket = socket_connected(&addr.local, &addr.remote);
        let rbuf = vec![0u8; 1500];
        let (upstream, _) = mpsc::unbounded_channel();
        PathTask { addr, timings: Timings::new(), rbuf, upstream, socket, stats }
    }

    pub async fn run(mut self, token: CancellationToken) {
        let mut interval_stats = interval(Duration::from_secs(1));
        let mut interval_ping = interval(Duration::from_secs(1));
        log::info!("Starting path task for {} <-> {}", self.addr.local, self.addr.remote);
        loop {
            select! {
                _ = token.cancelled() => break,
                _ = interval_stats.tick() => {
                    //log::info!("Path {} <-> {}: RTT = {:?}, Jitter = {:?}, Socket Error = {:?}", self.addr.local, self.addr.remote, self.timings.perceived_rtt, self.timings.perceived_jitter, self.socket.as_ref().err());
                    self.stats.send_modify(|stats| {
                        stats.rtt = self.timings.perceived_rtt;
                        stats.jitter = self.timings.perceived_jitter;
                        stats.error = self.socket.as_ref().err().map(|e| e.to_string());
                    });
                }
                _ = interval_ping.tick() => {
                    log::info!("Resetting path {} <-> {} after 30s", self.addr.local, self.addr.remote);
                    self.send_ping().await
                }
                _ = self.receive() => {},
            }
        }
    }

    async fn receive(&mut self) {
        if let Ok(socket) = &self.socket {
            if let Ok(rcvd) = socket.recv(&mut self.rbuf).await {
                let buf = &self.rbuf[..rcvd];
                if let Some(msg) = MsgTimings::decode(buf) {
                    self.timings.update(&msg);
                }
            }
        } else {
            std::future::pending().await
        }
    }

    async fn send_ping(&mut self) {
        if self.socket.is_err() {
            self.socket = socket_connected(&self.addr.local, &self.addr.remote);
        }
        if let Ok(socket) = &self.socket {
            let mut buf = [0u8; 1 + 4 * 8];
            let msg = self.timings.msg();
            let _ = msg.encode(&mut buf);
            if let Err(e) = socket.send(&buf).await {
                self.socket = Err(e);
                self.timings.reset();
            }
        }
    }
}

fn socket_connected(bind: &SocketAddrV6, conn: &SocketAddrV6) -> Result<UdpSocket, std::io::Error> {
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
