use crate::constants::*;
use crate::link::Link;
use crate::{Id, Node};
use std::collections::BTreeMap;
use std::net::SocketAddrV6;
use std::ops;
use std::sync::Arc;
use tokio::sync::watch::{self};
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
pub struct Peer {
    id: Id,
    links: watch::Sender<BTreeMap<(SocketAddrV6, SocketAddrV6), Arc<Link>>>,
    token: CancellationToken,
}

impl Peer {
    pub fn new(id: Id, token: CancellationToken) -> Arc<Self> {
        let links = watch::channel(BTreeMap::new()).0;
        let links_ = links.clone();
        let token_ = token.clone();
        tokio::spawn(async move {
            token_.cancelled().await;
            links_.send_modify(BTreeMap::clear);
        });
        Arc::new(Self { id, links, token })
    }

    pub fn id(&self) -> &Id {
        &self.id
    }

    pub fn is_empty(&self) -> bool {
        self.links.borrow().is_empty()
    }

    pub fn connect(self: &Arc<Self>, node: &Arc<Node>, addr: &SocketAddrV6) -> Arc<Link> {
        let mut conn = None;
        let key = (node.addr().clone(), addr.clone());

        self.links.send_if_modified(|m| {
            if let Some(c) = m.get(&key) {
                conn = Some(c.clone());
                false
            } else {
                let c = Link::new(node.clone(), self.clone(), addr.clone());
                conn = Some(c.clone());
                m.insert(key, c);
                true
            }
        });

        let conn = conn.unwrap();
        tokio::spawn({
            let conn = conn.clone();
            let peer = self.clone();
            async move {
                conn.token().cancelled().await;
                sleep(LINK_REMOVAL_DELAY).await;
                peer.links.send_modify(|m| m.remove(&key).map(drop).unwrap_or_default());
                if peer.links.borrow().is_empty() {
                    peer.token.cancel();
                }
            }
        });

        conn
    }

    pub fn links(&self) -> impl ops::Deref<Target = BTreeMap<(SocketAddrV6, SocketAddrV6), Arc<Link>>> + '_ {
        self.links.borrow()
    }

    pub fn token(&self) -> &CancellationToken {
        &self.token
    }
}
