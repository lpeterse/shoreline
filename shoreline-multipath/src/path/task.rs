use crate::timings::{MsgTimings, Timings};
use super::stats::PathStats;
use super::status::Status;
use tokio::net::UdpSocket;
use tokio::select;
use tokio::sync::{mpsc, watch};
use tokio::time::Duration;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;

pub struct PathTask {
    timings: Timings,
    rbuf: Vec<u8>,
    upstream: mpsc::UnboundedSender<Vec<u8>>,
    downstream: Option<mpsc::UnboundedReceiver<Vec<u8>>>,
    socket: UdpSocket,
    error: Option<String>,
    stats: watch::Sender<PathStats>,
    status_tx: watch::Sender<Status>,
}

impl PathTask {
    pub fn new(socket: UdpSocket, stats: watch::Sender<PathStats>, status_tx: watch::Sender<Status>) -> Self {
        let rbuf = vec![0u8; 1500];
        let (upstream, _) = mpsc::unbounded_channel();
        PathTask { timings: Timings::new(), rbuf, upstream, downstream: None, socket, stats, status_tx, error: None }
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
        match self.socket.recv(&mut self.rbuf).await {
            Ok(rcvd) => {
                let buf = &self.rbuf[..rcvd];
                if let Some(msg) = MsgTimings::decode(buf) {
                    self.error = None;
                    self.timings.update(&msg);
                }
            }
            Err(e) => {
                self.set_status(Status::Closed);
                self.error = Some(e.to_string());
                self.timings.reset();
            }
        }
    }

    async fn send_ping(&mut self) {
        let mut buf = [0u8; 1 + 4 * 8];
        let msg = self.timings.msg();
        let _ = msg.encode(&mut buf);
        if let Err(e) = self.socket.send(&buf).await {
            self.set_status(Status::Closed);
            self.error = Some(e.to_string());
            self.timings.reset();
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

    fn set_status(&mut self, status: Status) {
        self.status_tx.send_if_modified(|x| {
            std::mem::replace(x, status) != status
        });
    }
}
