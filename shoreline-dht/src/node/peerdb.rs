use crate::{PeerInfo};

pub trait PeerDB: Send + Sync + Clone +  'static {
    fn load(&self) -> impl Future<Output = Vec<PeerInfo>> + Send;
    fn store(&self, peers: Vec<PeerInfo>) -> impl Future<Output = ()> + Send;
}

impl PeerDB for () {
    async fn load(&self) -> Vec<PeerInfo> {
        vec![]
    }

    async fn store(&self, _peers: Vec<PeerInfo>){
        // no-op
    }
}
