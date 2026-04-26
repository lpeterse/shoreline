mod addr;
mod stats;
mod task;
mod status;

pub use self::addr::PathAddr;
pub use self::stats::PathStats;
pub use self::task::PathTask;
pub use self::status::Status;

use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

pub struct Path {
    stats: watch::Receiver<PathStats>,
    status : watch::Receiver<Status>,
    token: CancellationToken,
}

impl Path {
    pub fn new(addr: PathAddr, token: CancellationToken) -> Result<Self, std::io::Error> {
        let socket = addr.connect()?;
        let (stats_tx, stats_rx) = watch::channel(PathStats::new());
        let (status_tx, status_rx) = watch::channel(Status::Closed);
        let task = PathTask::new(socket, stats_tx, status_tx);
        let _ = tokio::spawn(task.run(token.clone()));
        Ok(Path { stats: stats_rx, status: status_rx, token })
    }

    pub fn status(&self) -> &watch::Receiver<Status> {
        &self.status
    }

    pub fn stats(&self) -> &watch::Receiver<PathStats> {
        &self.stats
    }
}

impl Drop for Path {
    fn drop(&mut self) {
        self.token.cancel();
    }
}
