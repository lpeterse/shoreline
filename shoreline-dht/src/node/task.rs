use super::super::common::Infos;
use super::super::common::Msg;
use super::super::{Id, Info, Link};
use super::cmd::Command;
use super::stat::NodeStat;
use crate::Error;
use crate::Node;
use crate::Peers;
use crate::constants::*;
use crate::util::socket_bound;
use bencode_minimal::Value;
use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::net::SocketAddrV6;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::sync::watch;
use tokio::task::JoinSet;
use tokio::time::{Interval, interval};

pub struct Task {
    node: Arc<Node>,
    sock: UdpSocket,
    stat: watch::Sender<NodeStat>,
    cmds: mpsc::UnboundedReceiver<Command>,
    intvl: Interval,
    peers: Peers,
    seeds: watch::Receiver<Vec<SocketAddrV6>>,
    table: BTreeMap<usize, BTreeMap<SocketAddrV6, Arc<Link>>>,
    infos: JoinSet<Infos>,
    terms: JoinSet<Arc<Link>>,
}

impl Task {
    pub fn spawn(
        node: Arc<Node>,
        peers: Peers,
        seeds: watch::Receiver<Vec<SocketAddrV6>>,
        stat: watch::Sender<NodeStat>,
        cmds: mpsc::UnboundedReceiver<Command>,
    ) -> Result<(), Error> {
        let sock = socket_bound(*node.addr()).map_err(Error::Socket)?;
        let this = Self {
            node,
            sock,
            stat,
            cmds,
            intvl: interval(REFRESH_INTERVAL),
            peers,
            seeds,
            table: BTreeMap::new(),
            infos: JoinSet::new(),
            terms: JoinSet::new(),
        };
        tokio::task::spawn(Box::new(this).run());
        Ok(())
    }

    /// The main loop of the node task
    ///
    /// This function runs until the node is shut down or an error occurs.
    async fn run(mut self: Box<Self>) {
        self.seed().await;
        self.run_loop().await;

        for bucket in self.table.into_values() {
            for link in bucket.into_values() {
                link.token().cancel();
            }
        }
    }

    async fn run_loop(&mut self) {
        let mut rbuf = vec![0u8; RBUF_SIZE];
        let mut sbuf = vec![0u8; RBUF_SIZE];

        loop {
            tokio::select! {
                res = self.sock.recv_from(&mut rbuf) => {
                    if let Ok((len, SocketAddr::V6(addr))) = res {
                        self.stat.send_modify(|s| s.add_rx_bytes(rbuf.len() as u64));
                        sbuf.clear();
                        self.dispatch(addr, &rbuf[..len], &mut sbuf).await;
                    }
                }
                Some(cmd) = self.cmds.recv() => {
                    match cmd {
                        Command::FindNode(id, tx) => {
                            let infos = self.find(&id);
                            let _ = tx.send(infos);
                        }
                        Command::Suggest(info) => self.suggest(info),
                    }
                }
                Some(res) = self.infos.join_next() => {
                    res.unwrap().iter().for_each(|i| self.suggest(*i));
                }
                Some(res) = self.terms.join_next() => {
                    self.remove(res.unwrap());
                }
                Ok(()) = self.seeds.changed() => {
                    self.seed().await;
                }
                _ = self.intvl.tick() => {
                    self.refresh();
                }
                _ = self.node.token().cancelled() => {
                    break;
                }
            }
        }
    }

    async fn seed(&mut self) {
        let addrs = self.seeds.borrow().clone();
        for addr in addrs.into_iter() {
            let target = Id::random();
            let buf = Msg::find_node_query(&[0], self.node.id(), &target).encode();
            self.send(&buf, addr).await;
        }
    }

    fn suggest(&mut self, info: Info) {
        if &info.id != self.node.id() && !info.id.is_null() {
            let bucket = self.node.id().similarity(&info.id);
            let bucket = self.table.entry(bucket).or_default();
            if !bucket.contains_key(&info.addr) && bucket.len() < BUCKET_MAX_LEN {
                let peer = self.peers.get(&info.id);
                let link = peer.connect(&self.node, &info.addr);
                bucket.insert(info.addr, link.clone());
                self.terms.spawn(async move {
                    link.token().cancelled().await;
                    link
                });
            }
        }
    }

    fn remove(&mut self, link: Arc<Link>) {
        let bucket = self.node.id().similarity(link.peer().id());
        if let Some(link) = self.table.get_mut(&bucket).and_then(|b| b.remove(link.addr())) {
            link.token().cancel();
        }
    }

    fn count(&self) -> usize {
        self.table.values().map(|b| b.len()).sum()
    }

    fn random(&self) -> Option<Arc<Link>> {
        let count = self.count();
        if count == 0 {
            return None;
        }
        let idx = rand::random::<u32>() as usize % count;
        let mut n = 0;
        for bucket in self.table.values() {
            for link in bucket.values() {
                if n == idx {
                    return Some(link.clone());
                }
                n += 1;
            }
        }
        None
    }

    fn refresh(&mut self) {
        let id = if self.count() < 16 {
            Id::random()
        } else {
            *self.node.id()
        };
        if let Some(peer) = self.random() {
            self.infos.spawn(async move { peer.find_node(&id).await.unwrap_or_default() });
        }
    }

    fn find(&self, target: &Id) -> Infos {
        let mut infos = vec![];
        let start = self.node.id().similarity(target);
        for bucket in start..160 {
            if let Some(b) = self.table.get(&bucket) {
                for link in b.values() {
                    infos.push(Info::new(*link.peer().id(), *link.addr()));
                    if infos.len() >= 8 {
                        break;
                    }
                }
            }
        }
        Infos::from(infos)
    }

    async fn dispatch(&mut self, addr: std::net::SocketAddrV6, rbuf: &[u8], sbuf: &mut Vec<u8>) -> Option<()> {
        let v = Value::decode(rbuf, BENCODE_MAX_ALLOCS)?;

        match v.get::<&str>(Msg::Y)? {
            Msg::Q => {
                let q = v.get::<&str>(Msg::Q)?;
                let a = v.get::<&Value>(Msg::A)?;
                let t = v.get::<&[u8]>(Msg::T)?;
                let id = a.get::<Id>(Msg::ID)?;
                match q {
                    Msg::PING => {
                        Msg::ping_response(t, self.node.id()).encode_into(sbuf);
                    }
                    Msg::FIND_NODE => {
                        let target = a.get::<Id>(Msg::TARGET)?;
                        let nodes6 = self.find(&target).encode();
                        Msg::find_node_response(t, self.node.id(), &nodes6).encode_into(sbuf);
                    }
                    Msg::GET_PEERS => {
                        let info_hash = a.get::<Id>(Msg::INFO_HASH)?;
                        let nodes6 = self.find(&info_hash).encode();
                        let token = Msg::TOKEN_VALUE.as_bytes();
                        Msg::get_peers_response(t, self.node.id(), token, &nodes6).encode_into(sbuf);
                    }
                    Msg::ANNOUNCE_PEER => {
                        Msg::announce_peer_response(t, self.node.id()).encode_into(sbuf);
                    }
                    _ => (),
                }
                self.suggest(Info::new(id, addr));
            }
            Msg::R => {
                let r = v.get::<&Value>(Msg::R)?;
                if let Some(infos) = r.get::<&[u8]>(Msg::NODES6).and_then(Infos::decode) {
                    infos.iter().for_each(|info| self.suggest(*info));
                }
            }
            _ => (),
        };

        if !sbuf.is_empty() {
            self.send(sbuf, addr).await?;
        }

        Some(())
    }

    async fn send(&mut self, sbuf: &[u8], addr: SocketAddrV6) -> Option<()> {
        let len = self.sock.send_to(&sbuf, addr).await.ok()?;
        self.stat.send_modify(|s| s.add_tx_bytes(len as u64));
        Some(())
    }
}
