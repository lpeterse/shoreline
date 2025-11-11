use super::super::Error;
use super::super::common::Infos;
use super::super::common::*;
use super::super::{Id, Info, Version};
use super::cmd::{CmdFindNode, CmdPing, PeerCmd};
use super::status::Status;
use super::trxs::Trxs;
use bencode_minimal::Value;
use super::super::node::NodeCmd;
use super::super::peer::stat::PeerStat;
use crate::util::{Backoff, check, socket_connected};
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::sync::{oneshot, watch};
use tokio::task::{JoinHandle, JoinSet};
use tokio::time::{Duration, Instant, Interval, interval_at};

const EPROTO: Error = Error::PeerProtocolViolation;

/// [PeerTask] handles communication with a single DHT peer
pub struct PeerTask {
    pinf: Info,
    ninf: Info,
    pcmd: mpsc::UnboundedReceiver<PeerCmd>,
    ncmd: mpsc::WeakUnboundedSender<NodeCmd>,
    stat: watch::Sender<PeerStat>,
    ping: Interval,
    trxs: Trxs,
    qrys: JoinSet<Result<Vec<u8>, Error>>,
    boff: Backoff,
}

impl PeerTask {
    const RBUF_SIZE: usize = 1500;

    pub const TIMEOUT_FACTOR: f32 = 3.0;
    pub const TIMEOUT_DEFAULT: Duration = Duration::from_secs(10);
    pub const PING_INTERVAL: Duration = Duration::from_secs(25);
    pub const PING_STARTUP_DELAY: Duration = Duration::from_millis(10);
    pub const RETRY_WAIT_MAX: Duration = Duration::from_secs(300);
    pub const BENCODE_MAX_ALLOCS: usize = 20;

    pub fn spawn(
        pinf: Info,
        ninf: Info,
        pcmd: mpsc::UnboundedReceiver<PeerCmd>,
        ncmd: mpsc::WeakUnboundedSender<NodeCmd>,
        stat: watch::Sender<PeerStat>,
    ) -> JoinHandle<()> {
        let s = Self {
            pinf,
            ninf,
            pcmd,
            ncmd,
            ping: interval_at(Instant::now() + Self::PING_STARTUP_DELAY, Self::PING_INTERVAL),
            trxs: Trxs::new(&stat),
            qrys: JoinSet::new(),
            boff: Backoff::new(Self::RETRY_WAIT_MAX),
            stat,
        };
        tokio::task::spawn(Box::new(s).run())
    }

    /// Run connect on the UDP socket pair and eventually call [Self::run_connected]
    ///
    /// This function will keep trying to connect with exponential backoff until
    /// successful. While not connected, it will reject incoming commands with
    /// [Error::PeerNotConnected].
    async fn run(mut self: Box<Self>) {
        'reconnect: loop {
            tokio::select! {
                // Attempt to connect after exponential backoff. The first attempt
                // succeeds immediately. Returns only on fatal socket error. On other
                // errors, it sets the error state and continues the loop.
                _ = self.boff.tick() => {
                    let bind = &self.ninf.addr;
                    let conn = &self.pinf.addr;
                    let sock = match socket_connected(bind, conn) {
                        Ok(s) => s,
                        Err(e) => {
                            self.set_fail(Error::Socket(e));
                            continue 'reconnect;
                        }
                    };
                    while let Err(e) = self.run_connected(&sock).await {
                        if matches!(e, Error::Socket(_)) {
                            self.set_fail(e);
                            continue 'reconnect;
                        } else {
                            self.set_fail(e);
                        }
                    }
                }
                // Reject incoming commands while not connected
                Some(cmd) = self.pcmd.recv() => {
                    cmd.reject(Error::PeerNotConnected);
                }
                // Reject timed-out transactions while not connected
                Some(cmd) = self.trxs.timeout_next() => {
                    cmd.reject(Error::PeerNotConnected);
                }
            }
        }
    }

    /// Run the communication on the connected UDP socket
    ///
    /// This function handles incoming and outgoing messages, timeouts,
    /// and periodic pings. It will only return with error.
    async fn run_connected(&mut self, sock: &UdpSocket) -> Result<(), Error> {
        log::error!("Connected to peer {}", self.pinf.addr);
        let mut rbuf = vec![0u8; Self::RBUF_SIZE];

        loop {
            tokio::select! {
                // Periodic ping if nothing else has been sent recently
                _ = self.ping.tick() => {
                    let cmd = CmdPing::new().0;
                    self.exec_ping(cmd, &sock).await?;
                }
                // Incoming message
                res = sock.recv(&mut rbuf) => {
                    let len = res.map_err(Error::Socket)?;
                    self.rcvd(&rbuf[..len], &sock).await?;
                }
                // Incoming command
                Some(cmd) = self.pcmd.recv() => {
                    self.exec(cmd, &sock).await?;
                }
                // Transaction timeout
                Some(cmd) = self.trxs.timeout_next() => {
                    cmd.reject(Error::PeerQueryTimeout);
                    self.set_miss();
                }
                // Completed query
                Some(res) = self.qrys.join_next() => {
                    let msg = res.unwrap()?;
                    self.send(&msg, &sock).await?;
                }
            }
        }
    }

    /// Handle received message (either query, response or error)
    async fn rcvd(&mut self, buf: &[u8], sock: &UdpSocket) -> Result<(), Error> {
        self.stat.send_modify(|s| {
            s.add_rx_packets(1);
            s.add_rx_bytes(buf.len() as u64);
        });
        let msg = Value::decode(buf, Self::BENCODE_MAX_ALLOCS).ok_or(Error::PeerBencodeInvalid)?;
        let y = msg.get::<&str>(Msg::Y).ok_or(EPROTO)?;
        self.set_version(msg.get::<Version>(Msg::V));
        match y {
            Msg::Q => self.rcvd_query(&msg, sock).await,
            Msg::R => self.rcvd_response(&msg).await,
            Msg::E => self.rcvd_error(&msg).await,
            _ => Err(EPROTO)?,
        }
    }

    /// Handle received query message
    /// 
    /// Checks the peer ID and throws an error on mismatch.
    async fn rcvd_query(&mut self, msg: &Value<'_>, sock: &UdpSocket) -> Result<(), Error> {
        let t = msg.get::<&[u8]>(Msg::T).ok_or(EPROTO)?;
        let q = msg.get::<&str>(Msg::Q).ok_or(EPROTO)?;
        let a = msg.get::<&Value<'_>>(Msg::A).ok_or(EPROTO)?;
        let pid = a.get::<Id>(Msg::ID).ok_or(Error::PeerIdMissing)?;
        check(pid == self.pinf.id).ok_or(Error::PeerIdMismatch)?;
        match q {
            Msg::PING => self.rcvd_query_ping(t, sock).await,
            Msg::FIND_NODE => self.rcvd_query_find_node(msg, t, sock).await,
            Msg::GET_PEERS => self.rcvd_query_get_peers(msg, t, sock).await,
            Msg::ANNOUNCE_PEER => self.rcvd_query_announce_peer(t, sock).await,
            _ => self.send(&Msg::error_204(t).encode(), sock).await,
        }
    }

    /// Handle received ping query
    async fn rcvd_query_ping(&mut self, t: &[u8], sock: &UdpSocket) -> Result<(), Error> {
        let r = Msg::ping_response(t, &self.ninf.id).encode();
        self.send(&r, sock).await
    }

    /// Handle received find_node query
    async fn rcvd_query_find_node(&mut self, msg: &Value<'_>, t: &[u8], sock: &UdpSocket) -> Result<(), Error> {
        let t = t.to_vec();
        let a = msg.get::<&Value<'_>>(Msg::A).ok_or(EPROTO)?;
        let target = a.get::<Id>(Msg::TARGET).ok_or(EPROTO)?;
        let id = self.ninf.id;
        let (tx, rx) = oneshot::channel();
        if self.ncmd.upgrade().and_then(|s| s.send(NodeCmd::FindNode(target, tx)).ok()).is_some() {
            self.qrys.spawn(async move {
                let n6 = rx.await.unwrap_or_default().encode();
                let m = Msg::find_node_response(&t, &id, &n6);
                Ok(m.encode())
            });
        } else {
            let m = Msg::find_node_response(&t, &id, &[]);
            self.send(&m.encode(), sock).await?;
        }
        Ok(())
    }

    /// Handle received get_peers query
    async fn rcvd_query_get_peers(&mut self, msg: &Value<'_>, t: &[u8], sock: &UdpSocket) -> Result<(), Error> {
        let t = t.to_vec();
        let a = msg.get::<&Value<'_>>(Msg::A).ok_or(EPROTO)?;
        let info_hash = a.get::<Id>(Msg::INFO_HASH).ok_or(EPROTO)?;
        let id = self.ninf.id;
        let (tx, rx) = oneshot::channel();
        if self.ncmd.upgrade().and_then(|s| s.send(NodeCmd::FindNode(info_hash, tx)).ok()).is_some() {
            self.qrys.spawn(async move {
                let n6 = rx.await.unwrap_or_default().encode();
                let m = Msg::get_peers_response(&t, &id, "FIXME".as_bytes(), &n6);
                Ok(m.encode())
            });
        } else {
            let m = Msg::get_peers_response(&t, &id, "FIXME".as_bytes(), &[]);
            self.send(&m.encode(), sock).await?;
        }
        Ok(())
    }

    /// Handle received announce_peer query
    async fn rcvd_query_announce_peer(&mut self, t: &[u8], sock: &UdpSocket) -> Result<(), Error> {
        let msg = Msg::announce_peer_response(&t, &self.ninf.id).encode();
        self.send(&msg, sock).await
    }

    /// Handle received response message
    ///
    /// Checks the peer ID and throws an error on mismatch.
    /// Dispatching a response looks up the corresponding transaction
    /// and calls the appropriate handler. The message is ignored if no
    /// matching transaction is found as this might happen on restart and is
    /// not necessarily an error. On successful handling, the peer is marked as good
    /// and any error is cleared and the exponential backoff reset.
    async fn rcvd_response(&mut self, msg: &Value<'_>) -> Result<(), Error> {
        let t = msg.get(Msg::T).map(u64::from_be_bytes).ok_or(EPROTO)?;
        let r = msg.get::<&Value<'_>>(Msg::R).ok_or(EPROTO)?;
        let pid = r.get::<Id>(Msg::ID).ok_or(Error::PeerIdMissing)?;
        check(self.pinf.id == pid || self.pinf.id == Id::UNKNOWN).ok_or(Error::PeerIdMismatch)?;
        if let Some(cmd) = self.trxs.resolve(t) {
            match cmd {
                PeerCmd::Ping(cmd) => self.rcvd_response_ping(cmd).await?,
                PeerCmd::FindNode(cmd) => self.rcvd_response_find_node(cmd, r).await?,
            }
            self.set_good();
        }
        Ok(())
    }

    /// Handle received ping response
    async fn rcvd_response_ping(&mut self, cmd: CmdPing) -> Result<(), Error> {
        let _ = cmd.response.send(Ok(()));
        Ok(())
    }

    /// Handle received find_node response
    async fn rcvd_response_find_node(&mut self, cmd: CmdFindNode, r: &Value<'_>) -> Result<(), Error> {
        let nodes6 = r.get::<&[u8]>(Msg::NODES6).ok_or(EPROTO)?;
        let nodes6 = Infos::decode(nodes6).ok_or(EPROTO)?;
        let _ = cmd.response.send(Ok(nodes6));
        Ok(())
    }

    /// Handle received error message
    async fn rcvd_error(&mut self, msg: &Value<'_>) -> Result<(), Error> {
        let tid = msg.get(Msg::T).map(u64::from_be_bytes).ok_or(EPROTO)?;
        if let Some(cmd) = self.trxs.resolve(tid) {
            let (code, msg) = msg.get::<(i64, &str)>(Msg::E).ok_or(EPROTO)?;
            let e = Error::PeerQueryError(code, msg.to_string());
            match cmd {
                PeerCmd::Ping(x) => {
                    let _ = x.response.send(Err(e));
                }
                PeerCmd::FindNode(x) => {
                    let _ = x.response.send(Err(e));
                }
            }
        }
        Ok(())
    }

    /// Execute outgoing command
    async fn exec(&mut self, cmd: PeerCmd, sock: &UdpSocket) -> Result<(), Error> {
        match cmd {
            PeerCmd::Ping(cmd) => self.exec_ping(cmd, sock).await,
            PeerCmd::FindNode(cmd) => self.exec_find_node(cmd, sock).await,
        }
    }

    /// Execute outgoing ping command
    async fn exec_ping(&mut self, cmd: CmdPing, sock: &UdpSocket) -> Result<(), Error> {
        let tid = self.trxs.start(cmd).to_be_bytes();
        let msg = Msg::ping_query(&tid, &self.ninf.id);
        let buf = msg.encode();
        self.send(&buf, sock).await
    }

    /// Execute outgoing find_node command
    async fn exec_find_node(&mut self, cmd: CmdFindNode, sock: &UdpSocket) -> Result<(), Error> {
        let tgt = cmd.target;
        let tid = self.trxs.start(cmd).to_be_bytes();
        let msg = Msg::find_node_query(&tid, &self.ninf.id, &tgt);
        let buf = msg.encode();
        self.send(&buf, sock).await
    }

    /// Send message on the UDP socket
    ///
    /// The ping timer is reset after sending.
    async fn send(&mut self, buf: &[u8], sock: &UdpSocket) -> Result<(), Error> {
        sock.send(buf).await.map_err(Error::Socket)?;
        self.stat.send_modify(|s| {
            s.add_tx_packets(1);
            s.add_tx_bytes(buf.len() as u64);
        });
        self.ping.reset();
        Ok(())
    }

    /// Set the peer version
    fn set_version(&self, version: Option<Version>) {
        if version.is_some() {
            self.stat.send_if_modified(|s| {
                let modified = s.version != version;
                s.version = version;
                modified
            });
        }
    }

    /// Set the peer status to [Status::Good], clear any error and reset backoff
    fn set_good(&mut self) {
        self.boff.reset();
        self.stat.send_modify(|s| {
            s.status = Status::Good;
            s.error = None
        });
    }

    /// Set the peer status to [Status::Miss] (only if not already [Status::Miss] or worse)
    fn set_miss(&self) {
        self.stat.send_if_modified(|s| {
            if s.status < Status::Miss {
                s.status = Status::Miss;
                true
            } else {
                false
            }
        });
    }

    /// Set the peer status to [Status::Fail] status and sets the error
    fn set_fail<E: Into<Error>>(&self, e: E) {
        self.stat.send_modify(|s| {
            s.status = Status::Fail;
            s.error = Some(Arc::new(e.into()))
        });
    }
}
