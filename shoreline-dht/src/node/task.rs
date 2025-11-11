use crate::NodeInfo;
use crate::PeerId;
use super::super::common::Infos;
use super::super::common::Msg;
use super::super::{Id, Info, Peer};
use super::cmd::NodeCmd;
use super::socket::Socket;
use super::stat::NodeStat;
use super::table::Table;
use bencode_minimal::Value;
use std::collections::BTreeMap;
use std::net::SocketAddrV6;
use std::sync::{Arc, Weak};
use tokio::sync::mpsc;
use tokio::sync::watch;
use tokio::task::{JoinHandle, JoinSet};
use tokio::time::{Duration, Interval, interval};

pub struct NodeTask {
    info: NodeInfo,
    stat: watch::Sender<NodeStat>,
    cmdr: mpsc::UnboundedReceiver<NodeCmd>,
    cmds: mpsc::WeakUnboundedSender<NodeCmd>,
    refresh: Interval,
    peers_xord: watch::Sender<BTreeMap<PeerId, Weak<Peer>>>,
    peers_info: JoinSet<Infos>,
    peers_init: JoinSet<Option<Arc<Peer>>>,
    peers_keep: Table,
}

impl NodeTask {
    const REFRESH_INTERVAL: Duration = Duration::from_secs(15);
    const BENCODE_MAX_ALLOCS: usize = 20;

    pub fn spawn(
        info: NodeInfo,
        stat: watch::Sender<NodeStat>,
        cmdr: mpsc::UnboundedReceiver<NodeCmd>,
        cmds: mpsc::WeakUnboundedSender<NodeCmd>,
        peers_watch: watch::Sender<BTreeMap<PeerId, Weak<Peer>>>,
    ) -> JoinHandle<()> {
        let s = Self {
            info,
            stat,
            cmdr,
            cmds,
            refresh: interval(Self::REFRESH_INTERVAL),
            peers_xord: peers_watch,
            peers_info: JoinSet::new(),
            peers_init: JoinSet::new(),
            peers_keep: Table::new(info.id),
        };
        tokio::task::spawn(Box::new(s).run())
    }

    /// The main loop of the node task
    ///
    /// This function runs until the node is shut down or an error occurs.
    async fn run(mut self: Box<Self>) {
        self.run_seeding().await;
        self.run_loop().await;
    }

    async fn run_seeding(&mut self) {
        for seed in crate::SEEDS {
            if let Ok(addrs) = tokio::net::lookup_host(seed).await {
                for addr in addrs {
                    if let std::net::SocketAddr::V6(addrv6) = addr {
                        self.seed(addrv6);
                    }
                }
            }
        }
    }

    async fn run_loop(&mut self) {
        let mut socket = Socket::new(self.info.addr, &self.stat);

        loop {
            tokio::select! {
                _ = socket.receive() => {
                    socket.dispatch(|x,y,z| self.dispatch(x,y,z)).await;
                }
                Some(cmd) = self.cmdr.recv() => {
                    match cmd {
                        NodeCmd::Seed(addr) => {
                            self.seed(addr);
                        }
                        NodeCmd::GetNode(info, tx) => {
                            let peer = self.get(info);
                            let _ = tx.send(peer);
                        },
                        NodeCmd::GetNodes(tx) => {
                            let peers = self.peers_keep.collect();
                            let _ = tx.send(peers);
                        },
                        NodeCmd::FindNode(id, tx) => {
                            let peers = self.peers_keep.closest_n(&id, 8);
                            let infos = peers.iter().map(|p| *p.info()).collect::<Vec<_>>();
                            let _ = tx.send(Infos::from(infos));
                        }
                        NodeCmd::RemoveNode(node) => self.remove(node),
                        NodeCmd::SuggestNode(info) => self.suggest(info),
                    }
                }
                Some(res) = self.peers_info.join_next() => {
                    res.unwrap().iter().for_each(|i| self.suggest(*i));
                    self.schedule_refresh();
                }
                Some(res) = self.peers_init.join_next() => {
                    res.unwrap().into_iter().for_each(|p| self.peers_keep.insert(p));
                    self.schedule_refresh();
                }
                _ = self.refresh.tick() => {
                    self.refresh();
                }
            }
        }
    }

    fn get(&mut self, info: Info) -> Arc<Peer> {
        let xord = self.info.id.xor(&info.id);
        if let Some(p) = self.peers_xord.borrow().get(&xord).and_then(|w| w.upgrade()) {
            p
        } else {
            let p = Peer::new(info, self.info, self.cmds.clone());
            let q = p.clone();
            let _ = self.peers_xord.send_modify(|m| {
                m.insert(xord, Arc::downgrade(&p));
            });
            let _ = self.peers_init.spawn(async move { q.init().await.ok().filter(|s| s.is_good()).map(|_| q) });
            p
        }
    }

    fn find(&self, target: &Id, n: usize) -> Infos {
        self.peers_keep.closest_n(target, n).iter().map(|n| *n.info()).collect::<Vec<_>>().into()
    }

    fn remove(&mut self, id: Id) {
        let xord = self.info.id.xor(&id);
        let _ = self.peers_xord.send_modify(|m| {
            m.remove(&xord);
        });
        let _ = self.peers_keep.remove(&id);
    }

    fn suggest(&mut self, info: Info) {
        let xord = self.info.id.xor(&info.id);
        if self.info.id != info.id && !self.peers_xord.borrow().contains_key(&xord) {
            let p = Peer::new(info, self.info, self.cmds.clone());
            let _ = self.peers_xord.send_modify(|m| {
                m.insert(xord, Arc::downgrade(&p));
            });
            let _ = self.peers_init.spawn(async move { p.init().await.ok().filter(|s| s.is_good()).map(|_| p) });
        }
    }

    fn schedule_refresh(&mut self) {
        if self.peers_info.is_empty() && self.peers_init.is_empty() {
            let n = self.peers_keep.count_good();
            if n > 0 {
                let d = Duration::from_millis(1000 * n as u64);
                self.refresh.reset_after(d);
                return;
            }
        }
        self.refresh.reset();
    }

    fn seed(&mut self, addr: SocketAddrV6) {
        let info = Info::new(Id::UNKNOWN, addr);
        let peer = Peer::new(info, self.info, self.cmds.clone());
        let this = self.info.id;
        self.peers_info.spawn(async move { peer.find_node(&this).await.unwrap_or_default() });
    }

    fn refresh(&mut self) {
        let xord = self.peers_xord.borrow();
        let id = if xord.len() < 16 {
            Id::random()
        } else {
            self.info.id
        };
        if xord.len() > 0 {
            let r = rand::random_range(0..xord.len());
            if let Some(peer) = xord.values().nth(r).map(Weak::upgrade).flatten() {
                self.peers_info.spawn(async move { peer.find_node(&id).await.unwrap_or_default() });
            }
        }
    }

    fn dispatch(&mut self, addr: &std::net::SocketAddrV6, rbuf: &[u8], sbuf: &mut Vec<u8>) -> Option<()> {
        let v = Value::decode(rbuf, Self::BENCODE_MAX_ALLOCS)?;

        let _ = v.get::<&str>(Msg::Y).filter(|y| *y == Msg::Q)?;
        let t = v.get::<&[u8]>(Msg::T)?;
        let q = v.get::<&str>(Msg::Q)?;
        let a = v.get::<&Value>(Msg::A)?;
        let id = a.get::<Id>(Msg::ID)?;

        match q {
            Msg::PING => {
                Msg::ping_response(t, &self.info.id).encode_into(sbuf);
            }
            Msg::FIND_NODE => {
                let target = a.get::<Id>(Msg::TARGET)?;
                let nodes6 = self.find(&target, 8).encode();
                Msg::find_node_response(t, &self.info.id, &nodes6).encode_into(sbuf);
            }
            Msg::GET_PEERS => {
                let info_hash = a.get::<Id>(Msg::INFO_HASH)?;
                let nodes6 = self.find(&info_hash, 8).encode();
                let token = b"FIXME"; // FIXME: implement tokens
                Msg::get_peers_response(t, &self.info.id, token, &nodes6).encode_into(sbuf);
            }
            Msg::ANNOUNCE_PEER => {
                Msg::announce_peer_response(t, &self.info.id).encode_into(sbuf);
            }
            _ => (),
        }

        self.suggest(Info::new(id, *addr));
        Some(())
    }
}
