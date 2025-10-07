use crate::util::{Backoff, socket_bound};
use std::net::{SocketAddr, SocketAddrV6};
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::watch;
use tokio::time::Duration;
use super::super::Error;
use super::stat::NodeStat;

pub struct Socket {
    addr: SocketAddrV6,
    back: Backoff,
    stat: watch::Sender<NodeStat>,
    rbuf: Vec<u8>,
    sbuf: Vec<u8>,
    sock: Option<UdpSocket>,
    rcvd: Option<(usize, SocketAddrV6)>,
}

impl Socket {
    const RXBUF_SIZE: usize = 1500;
    const TXBUF_SIZE: usize = 1500;

    pub fn new(addr: SocketAddrV6, stat: &watch::Sender<NodeStat>) -> Self {
        Self {
            addr,
            back: Backoff::new(Duration::from_secs(60)),
            stat: stat.clone(),
            rbuf: vec![0u8; Self::RXBUF_SIZE],
            sbuf: Vec::with_capacity(Self::TXBUF_SIZE),
            sock: None,
            rcvd: None,
        }
    }

    pub async fn receive(&mut self) {
        while self.sock.is_none() {
            self.back.tick().await;
            match socket_bound(self.addr) {
                Ok(s) => self.sock = Some(s),
                Err(e) => self.stat.send_modify(|s| s.error = Some(Arc::new(Error::Socket(e))))
            }
        }
        let sock = self.sock.as_ref().unwrap();
        if let Ok((len, addr)) = sock.recv_from(&mut self.rbuf).await {
            self.stat.send_modify(|s| {
                s.add_rx_bytes(len as u64);
                s.add_rx_packets(1);
                s.error = None
            });
            if let SocketAddr::V6(addr) = addr {
                self.rcvd = Some((len, addr));
            }
        }
    }

    pub async fn dispatch<F>(&mut self, mut f: F)
    where
        F: FnMut(&SocketAddrV6, &[u8], &mut Vec<u8>) -> Option<()>,
    {
        if let (Some((len, addr)), Some(sock)) = (self.rcvd.take(), self.sock.as_ref()) {
            if f(&addr, &self.rbuf[..len], &mut self.sbuf).is_some() && !self.sbuf.is_empty() {
                let _ = sock.send_to(&self.sbuf, addr).await;
                self.stat.send_modify(|s| {
                    s.add_tx_bytes(self.sbuf.len() as u64);
                    s.add_tx_packets(1);
                });
            }
        }
    }
}
