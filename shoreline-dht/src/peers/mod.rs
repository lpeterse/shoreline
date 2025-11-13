use crate::{Id, peer::Peer};
use std::collections::BTreeMap;
use std::ops::Deref;
use std::sync::Arc;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub struct Peers {
    ctok: CancellationToken,
    peers: watch::Sender<BTreeMap<Id, Arc<Peer>>>,
}

impl Peers {
    pub fn new(ctok: CancellationToken) -> Self {
        let (peers, _) = watch::channel(BTreeMap::new());
        let ctok_ = ctok.clone();
        let peers_ = peers.clone();
        tokio::spawn(async move {
            ctok_.cancelled().await;
            peers_.send_modify(BTreeMap::clear);
        });
        Self { ctok, peers }
    }

    pub fn get(&self, id: &Id) -> Arc<Peer> {
        if let Some(peer) = self.peers.borrow().get(id) {
            peer.clone()
        } else {
            let id = *id;
            let ctok = self.ctok.child_token();
            let peer = Peer::new(id, ctok.clone());
            let peers = self.peers.clone();
            self.peers.send_modify(|m| {
                m.insert(id, peer.clone());
            });
            tokio::spawn(async move {
                ctok.cancelled().await;
                peers.send_modify(|m| {
                    m.remove(&id);
                });
            });
            peer
        }
    }

    pub fn borrow(&self) -> impl Deref<Target = BTreeMap<Id, Arc<Peer>>> + '_ {
        self.peers.borrow()
    }

    // pub fn subscribe(&self) -> watch::Receiver<BTreeMap<Id, Arc<Peer>>> {
    //     self.peers.subscribe()
    // }
    pub fn ctok(&self) -> &CancellationToken {
        &self.ctok
    }
}
