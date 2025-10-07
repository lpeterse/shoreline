use super::super::peer::stat::PeerStat;
use crate::util::check;
use std::collections::BTreeMap;
use tokio::sync::watch;
use tokio::time::{Duration, Instant, sleep_until};

use super::cmd::PeerCmd;
use super::task::PeerTask;

pub struct Trxs {
    txid: u64,
    stat: watch::Sender<PeerStat>,
    queue: BTreeMap<u64, (Instant, PeerCmd)>,
    timeout: Instant,
}

impl Trxs {
    pub fn new(stat: &watch::Sender<PeerStat>) -> Self {
        Self { txid: 0, stat: stat.clone(), queue: BTreeMap::new(), timeout: Instant::now() }
    }

    pub fn start<T: Into<PeerCmd>>(&mut self, cmd: T) -> u64 {
        self.txid += 1;
        self.queue.insert(self.txid, (Instant::now(), cmd.into()));
        self.set_timeout();
        self.txid
    }

    pub fn resolve(&mut self, id: u64) -> Option<PeerCmd> {
        let (created, cmd) = self.queue.remove(&id)?;
        let rtt = created.elapsed();
        self.set_rtt(rtt);
        self.set_timeout();
        Some(cmd)
    }

    pub async fn timeout_next(&mut self) -> Option<PeerCmd> {
        check(!self.queue.is_empty())?;
        sleep_until(self.timeout).await;
        let (_, (_, cmd)) = self.queue.pop_first()?;
        self.set_timeout();
        Some(cmd)
    }

    fn set_rtt(&mut self, rtt: Duration) {
        self.stat.send_modify(|x| {
            let r = 0.5;
            let f = |d: Duration| d.mul_f32(r) + rtt.mul_f32(1.0 - r);
            x.rtt = x.rtt.take().map(f).or(Some(rtt));
        });
    }

    fn set_timeout(&mut self) {
        if let Some((_, (created, _))) = self.queue.first_key_value() {
            self.timeout = *created + self.current_timeout_duration();
        }
    }

    /// Get the current timeout duration based on the RTT statistics
    ///
    /// The timeout duration is calculated as rolling average RTT * TIMEOUT_FACTOR.
    /// If no RTT statistics are available, a default INIT_TIMEOUT is used.
    fn current_timeout_duration(&self) -> Duration {
        self.stat
            .borrow()
            .rtt
            .map(|x| x.mul_f32(PeerTask::TIMEOUT_FACTOR))
            .unwrap_or(PeerTask::TIMEOUT_DEFAULT)
    }
}
