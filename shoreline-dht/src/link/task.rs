use super::super::Error;
use super::super::common::Infos;
use super::super::common::*;
use super::super::{Id, Version};
use super::cmd::{CmdFindNode, CmdPing, Command};
use super::status::Status;
use super::trxs::Trxs;
use crate::constants::*;
use crate::link::stat::Stat;
use crate::util::{check, socket, socket_connected};
use crate::{Node, Peer};
use bencode_minimal::Value;
use std::net::SocketAddrV6;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::sync::watch;
use tokio::task::JoinSet;
use tokio::time::{Instant, Interval, interval_at};
use tokio_util::sync::CancellationToken;

const EPROTO: Error = Error::ProtocolViolation;

/// [PeerTask] handles communication with a single DHT peer
pub struct Task {
    node: Arc<Node>,
    peer: Arc<Peer>,
    addr: SocketAddrV6,
    sock: UdpSocket,
    ping: Interval,
    trxs: Trxs,
    qrys: JoinSet<Result<Vec<u8>, Error>>,
    cmds: mpsc::UnboundedReceiver<Command>,
    stat: watch::Sender<Stat>,
    token: CancellationToken,
}

impl Task {
    pub fn spawn(
        node: Arc<Node>,
        peer: Arc<Peer>,
        addr: SocketAddrV6,
        cmds: mpsc::UnboundedReceiver<Command>,
        stat: watch::Sender<Stat>,
    ) -> CancellationToken {
        let ctok = peer.token().child_token();
        let this = Self {
            node,
            peer,
            addr,
            sock: socket(),
            ping: interval_at(Instant::now() + PING_STARTUP_DELAY, PING_INTERVAL),
            trxs: Trxs::new(&stat),
            qrys: JoinSet::new(),
            cmds,
            stat,
            token: ctok.clone(),
        };
        tokio::task::spawn(Box::new(this).run());
        ctok
    }

    /// Run connect on the UDP socket pair and eventually call [Self::run_connected]
    ///
    /// This function will keep trying to connect with exponential backoff until
    /// successful. While not connected, it will reject incoming commands with
    /// [Error::PeerNotConnected].
    async fn run(mut self: Box<Self>) {
        if let Err(e) = self.run_().await {
            self.set_fail(e);
        }
        self.set_term();
        self.token.cancel();
    }

    /// Run the communication on the connected UDP socket
    ///
    /// This function handles incoming and outgoing messages, timeouts,
    /// and periodic pings. It will only return with error.
    async fn run_(&mut self) -> Result<(), Error> {
        let bind = self.node.addr();
        let conn = self.addr;
        self.sock = socket_connected(bind, &conn).map_err(Error::Socket)?;

        let mut rbuf = vec![0u8; RBUF_SIZE];

        loop {
            tokio::select! {
                // Periodic ping if nothing else has been sent recently
                _ = self.ping.tick() => {
                    let cmd = CmdPing::new().0;
                    self.exec_ping(cmd).await?;
                }
                // Incoming message
                res = self.sock.recv(&mut rbuf) => {
                    let len = res.map_err(Error::Socket)?;
                    self.rcvd(&rbuf[..len]).await?;
                }
                // Incoming command
                Some(cmd) = self.cmds.recv() => {
                    self.exec(cmd).await?;
                }
                // Transaction timeout
                Some(cmd) = self.trxs.timeout_next() => {
                    self.timeout(cmd)?;
                }
                // Completed query
                Some(res) = self.qrys.join_next() => {
                    let msg = res.unwrap()?;
                    self.send(&msg).await?;
                }
                _ = self.token.cancelled() => {
                    return Ok(())
                }
                _ = self.node.token().cancelled() => {
                    return Ok(())
                }
            }
        }
    }

    fn timeout(&mut self, cmd: Command) -> Result<(), Error> {
        let stat = self.stat.borrow();
        let status = stat.status;
        let elapsed = stat.rx_last.elapsed();
        drop(stat);
        if status == Status::Init {
            cmd.reject(Error::InitTimeout);
            self.set_fail(Error::InitTimeout);
            Err(Error::InitTimeout)
        } else if elapsed > TIMEOUT_TOTAL {
            cmd.reject(Error::TotalTimeout);
            self.set_fail(Error::TotalTimeout);
            Err(Error::TotalTimeout)
        } else {
            cmd.reject(Error::QueryTimeout);
            self.set_fail(Error::QueryTimeout);
            Ok(())
        }
    }

    /// Handle received message (either query, response or error)
    async fn rcvd(&mut self, buf: &[u8]) -> Result<(), Error> {
        self.stat.send_modify(|s| s.add_rx_bytes(buf.len() as u64));
        let msg = Value::decode(buf, BENCODE_MAX_ALLOCS).ok_or(Error::BencodeInvalid)?;
        let y = msg.get::<&str>(Msg::Y).ok_or(EPROTO)?;
        self.set_version(msg.get::<Version>(Msg::V));
        match y {
            Msg::Q => self.rcvd_query(&msg).await,
            Msg::R => self.rcvd_response(&msg).await,
            Msg::E => self.rcvd_error(&msg).await,
            _ => Err(EPROTO)?,
        }
    }

    /// Handle received query message
    ///
    /// Checks the peer ID and throws an error on mismatch.
    async fn rcvd_query(&mut self, msg: &Value<'_>) -> Result<(), Error> {
        let t = msg.get::<&[u8]>(Msg::T).ok_or(EPROTO)?;
        let q = msg.get::<&str>(Msg::Q).ok_or(EPROTO)?;
        let a = msg.get::<&Value<'_>>(Msg::A).ok_or(EPROTO)?;
        let pid = a.get::<Id>(Msg::ID).ok_or(Error::IdMissing)?;
        check(&pid == self.peer.id()).ok_or(Error::IdMismatch)?;
        match q {
            Msg::PING => self.rcvd_query_ping(t).await,
            Msg::FIND_NODE => self.rcvd_query_find_node(msg, t).await,
            Msg::GET_PEERS => self.rcvd_query_get_peers(msg, t).await,
            Msg::ANNOUNCE_PEER => self.rcvd_query_announce_peer(t).await,
            _ => self.send(&Msg::error_204(t).encode()).await,
        }
    }

    /// Handle received ping query
    async fn rcvd_query_ping(&mut self, t: &[u8]) -> Result<(), Error> {
        let r = Msg::ping_response(t, self.node.id()).encode();
        self.send(&r).await
    }

    /// Handle received find_node query
    async fn rcvd_query_find_node(&mut self, msg: &Value<'_>, t: &[u8]) -> Result<(), Error> {
        let t = t.to_vec();
        let a = msg.get::<&Value<'_>>(Msg::A).ok_or(EPROTO)?;
        let target = a.get::<Id>(Msg::TARGET).ok_or(EPROTO)?;
        let node = self.node.clone();
        self.qrys.spawn(async move {
            let n6 = node.find(&target).await.unwrap_or_default();
            let n6 = Infos::from(n6).encode();
            let m = Msg::find_node_response(&t, node.id(), &n6);
            Ok(m.encode())
        });
        Ok(())
    }

    /// Handle received get_peers query
    async fn rcvd_query_get_peers(&mut self, msg: &Value<'_>, t: &[u8]) -> Result<(), Error> {
        let t = t.to_vec();
        let a = msg.get::<&Value<'_>>(Msg::A).ok_or(EPROTO)?;
        let info_hash = a.get::<Id>(Msg::INFO_HASH).ok_or(EPROTO)?;
        let node = self.node.clone();
        self.qrys.spawn(async move {
            let n6 = node.find(&info_hash).await.unwrap_or_default();
            let n6 = Infos::from(n6).encode();
            let m = Msg::get_peers_response(&t, node.id(), Msg::TOKEN_VALUE.as_bytes(), &n6);
            Ok(m.encode())
        });
        Ok(())
    }

    /// Handle received announce_peer query
    async fn rcvd_query_announce_peer(&mut self, t: &[u8]) -> Result<(), Error> {
        let msg = Msg::announce_peer_response(&t, self.node.id()).encode();
        self.send(&msg).await
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
        let pid = r.get::<Id>(Msg::ID).ok_or(Error::IdMissing)?;
        check(self.peer.id() == &pid).ok_or(Error::IdMismatch)?;
        if let Some(cmd) = self.trxs.resolve(t) {
            match cmd {
                Command::Ping(cmd) => self.rcvd_response_ping(cmd).await?,
                Command::FindNode(cmd) => self.rcvd_response_find_node(cmd, r).await?,
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
            let e = Error::QueryError(code, msg.to_string());
            match cmd {
                Command::Ping(x) => {
                    let _ = x.response.send(Err(e));
                }
                Command::FindNode(x) => {
                    let _ = x.response.send(Err(e));
                }
            }
        }
        Ok(())
    }

    /// Execute outgoing command
    async fn exec(&mut self, cmd: Command) -> Result<(), Error> {
        match cmd {
            Command::Ping(cmd) => self.exec_ping(cmd).await,
            Command::FindNode(cmd) => self.exec_find_node(cmd).await,
        }
    }

    /// Execute outgoing ping command
    async fn exec_ping(&mut self, cmd: CmdPing) -> Result<(), Error> {
        let tid = self.trxs.start(cmd).to_be_bytes();
        let msg = Msg::ping_query(&tid, self.node.id());
        let buf = msg.encode();
        self.send(&buf).await
    }

    /// Execute outgoing find_node command
    async fn exec_find_node(&mut self, cmd: CmdFindNode) -> Result<(), Error> {
        let tgt = cmd.target;
        let tid = self.trxs.start(cmd).to_be_bytes();
        let msg = Msg::find_node_query(&tid, self.node.id(), &tgt);
        let buf = msg.encode();
        self.send(&buf).await
    }

    /// Send message on the UDP socket
    ///
    /// The ping timer is reset and the bytes sent are accounted for after sending.
    async fn send(&mut self, buf: &[u8]) -> Result<(), Error> {
        self.ping.reset();
        self.sock.send(buf).await.map_err(Error::Socket)?;
        self.stat.send_modify(|s| {
            s.add_tx_bytes(buf.len() as u64);
        });
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
        self.stat.send_modify(|s| {
            s.status = Status::Good;
            s.rx_last = Instant::now();
            s.error = None;
        });
    }

    /// Set the peer status to [Status::Fail] status and sets the error
    fn set_fail<E: Into<Error>>(&self, e: E) {
        self.stat.send_modify(|s| {
            s.status = Status::Fail;
            s.error = Some(Arc::new(e.into()))
        });
    }

    fn set_term(&self) {
        self.stat.send_modify(|s| {
            s.status = Status::Term;
        });
    }
}
