use std::fmt::Write;
use std::path::{Path, PathBuf};
use shoreline_dht::{PeerDB, PeerInfo};

#[derive(Debug, Clone)]
pub struct PeerFile {
    path: PathBuf,
}

impl PeerFile {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self { path: path.as_ref().to_path_buf() }
    }

    async fn load_err(&self) -> Result<Vec<PeerInfo>, std::io::Error> {
        let data = tokio::fs::read_to_string(&self.path ).await?;
        let peers = data
            .lines()
            .filter_map(|l| l.split_once(','))
            .filter_map(|(i, a)| Some(PeerInfo::new(i.trim().parse().ok()?, a.trim().parse().ok()?)))
            .collect();
        Ok(peers)
    }

    pub async fn store_err(&self, peers: Vec<PeerInfo>) -> Result<(), std::io::Error> {
        let mut data = String::new();
        for peer in peers {
            let _ = data.write_fmt(format_args!("{},{}\n", peer.id, peer.addr));
        }
        tokio::fs::write(&self.path, data).await?;
        Ok(())
    }
}

impl PeerDB for PeerFile {
    async fn load(&self) -> Vec<PeerInfo>{
        match self.load_err().await {
            Ok(peers) => peers,
            Err(e) => {
                log::warn!("Failed to load peers from file: {}", e);
                Vec::new()
            }
        }
    }

    async fn store(&self, peers: Vec<PeerInfo>) {
        if let Err(e) = self.store_err(peers).await {
            log::warn!("Failed to store peers to file: {}", e);
        }
    }
}
