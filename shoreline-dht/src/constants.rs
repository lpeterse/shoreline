use tokio::time::Duration;

pub const TIMEOUT_FACTOR: f32 = 3.0;
pub const TIMEOUT_INIT: Duration = Duration::from_secs(10);
pub const TIMEOUT_TOTAL: Duration = Duration::from_secs(300);

pub const RBUF_SIZE: usize = 1500;
pub const PING_INTERVAL: Duration = Duration::from_secs(25);
pub const PING_STARTUP_DELAY: Duration = Duration::from_millis(10);
pub const LINK_REMOVAL_DELAY: Duration = Duration::from_secs(5);
pub const BENCODE_MAX_ALLOCS: usize = 20;

pub const REFRESH_INTERVAL: Duration = Duration::from_secs(5);
pub const BUCKET_MAX_LEN: usize = 8;
