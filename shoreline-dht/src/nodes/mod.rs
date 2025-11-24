use crate::net::Netwatch;
use crate::{Id, Node, Peers};
use std::collections::BTreeMap;
use std::net::SocketAddrV6;
use std::ops::Deref;
use std::sync::Arc;
use tokio::select;
use tokio::sync::watch;

#[derive(Debug, Clone)]
pub struct Nodes {
    nodes: watch::Sender<BTreeMap<String, Arc<Node>>>,
}

impl Nodes {
    pub fn new(id: Id, port: u16, peers: Peers, seeds: watch::Receiver<Vec<SocketAddrV6>>) -> Self {
        let nodes = watch::channel(BTreeMap::new()).0;
        tokio::spawn(Self::run(id, port, peers.clone(), nodes.clone(), seeds));
        Self { nodes }
    }

    pub fn borrow(&self) -> impl Deref<Target = BTreeMap<String, Arc<Node>>> + '_ {
        self.nodes.borrow()
    }

    async fn run(
        id: Id,
        port: u16,
        peers: Peers,
        nodes: watch::Sender<BTreeMap<String, Arc<Node>>>,
        seeds: watch::Receiver<Vec<SocketAddrV6>>,
    ) {
        let token = peers.ctok().clone();
        let mut netwatch = Netwatch::new();
        loop {
            select! {
                _ = token.cancelled() => {
                    let m = nodes.send_replace(BTreeMap::new());
                    m.into_values().for_each(|n| n.token().cancel());
                    break;
                }
                _ = netwatch.changed() => {
                    let desired = netwatch.list();
                    nodes.send_modify(|m| {
                        for (k,v) in std::mem::take(m).into_iter() {
                            match desired.get(&k) {
                                Some(addr) if addr == v.addr().ip() => { m.insert(k, v); },
                                _ => v.token().cancel(),
                            };
                        };
                        for (interface, addr) in desired {
                            if !m.contains_key(&interface) {
                                let addr = SocketAddrV6::new(addr, port, 0, 0);
                                if let Ok(node) = Node::new(id, interface.clone(), addr, peers.clone(), seeds.clone()) {
                                    m.insert(interface, node);
                                }
                            }
                        }
                    });
                }
            }
        }
    }
}
