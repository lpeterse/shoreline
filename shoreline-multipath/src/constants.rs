use tokio::time::Duration;

pub const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(15);
pub const KEEPALIVE_TIMEOUT: Duration = Duration::from_secs(45);
