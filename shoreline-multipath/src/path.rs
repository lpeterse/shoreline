use super::addr::SocketAddrPair;
use super::stats::PathStats;
use super::timings::{MsgTimings, Timings};
use std::net::{Ipv6Addr, SocketAddr, SocketAddrV6};
use tokio::net::UdpSocket;
use tokio::select;
use tokio::sync::{mpsc, watch};
use tokio::time::Duration;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;

pub struct Path {
    stats: watch::Receiver<PathStats>,
    token: CancellationToken,
}

impl Path {
    pub fn new(addr: SocketAddrPair, token: CancellationToken) -> Self {
        let (stats_tx, stats_rx) = watch::channel(PathStats::new());
        let task = PathTask::new(addr, stats_tx);
        let _ = tokio::spawn(task.run(token.clone()));
        Path { stats: stats_rx, token }
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
    socket: Option<UdpSocket>,
    error: Option<String>,
    stats: watch::Sender<PathStats>,
}

impl PathTask {
    pub fn new(addr: SocketAddrPair, stats: watch::Sender<PathStats>) -> Self {
        let (socket, error) = match socket_connected(&addr.local, &addr.remote) {
            Ok(socket) => (Some(socket), None),
            Err(e) => (None, Some(e.to_string())),
        };
        let rbuf = vec![0u8; 1500];
        let (upstream, _) = mpsc::unbounded_channel();
        PathTask { addr, timings: Timings::new(), rbuf, upstream, socket, stats, error }
    }

    pub async fn run(mut self, token: CancellationToken) {
        let mut interval_stats = interval(Duration::from_secs(1));
        let mut interval_ping = interval(Duration::from_secs(1));
        loop {
            select! {
                _ = token.cancelled() => break,
                _ = interval_stats.tick() => {
                    self.update_stats();
                }
                _ = interval_ping.tick() => {
                    self.send_ping().await
                }
                _ = self.receive() => {},
            }
        }
    }

    async fn receive(&mut self) {
        if let Some(socket) = &self.socket {
            match socket.recv(&mut self.rbuf).await {
                Ok(rcvd) => {
                    let buf = &self.rbuf[..rcvd];
                    if let Some(msg) = MsgTimings::decode(buf) {
                        self.error = None;
                        self.timings.update(&msg);
                    }
                }
                Err(e) => {
                    self.error = Some(e.to_string());
                    self.timings.reset();
                }
            }
            // if let Ok(rcvd) = socket.recv(&mut self.rbuf).await {
            //     let buf = &self.rbuf[..rcvd];
            //     if let Some(msg) = MsgTimings::decode(buf) {
            //         self.timings.update(&msg);
            //     }
            // }
        } else {
            std::future::pending().await
        }
    }

    async fn send_ping(&mut self) {
        if let Some(socket) = &self.socket {
            let mut buf = [0u8; 1 + 4 * 8];
            let msg = self.timings.msg();
            let _ = msg.encode(&mut buf);
            if let Err(e) = socket.send(&buf).await {
                self.error = Some(e.to_string());
                self.timings.reset();
            }
        }
    }

    fn update_stats(&mut self) {
        self.stats.send_modify(|stats| {
            stats.rtt = self.timings.measured_rtt;
            stats.jitter = self.timings.measured_jitter;
            stats.latest_rx = self.timings.latest_rx;
            stats.error = self.error.clone();
        });
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
