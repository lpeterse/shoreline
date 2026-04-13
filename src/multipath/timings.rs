use tokio::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct Timings {
    pub local_clock: Instant,
    pub remote_clock: Instant,
    pub last_delta: Duration,
    pub perceived_rtt: Duration,
    pub perceived_jitter: Duration,
    pub reported_rtt: Duration,
    pub reported_jitter: Duration,
}

impl Timings {
    const IIR_RATIO: f64 = 0.9;

    pub fn new() -> Self {
        Self {
            local_clock: Instant::now(),
            remote_clock: Instant::now(),
            last_delta: Duration::from_nanos(0),
            perceived_rtt: Duration::from_nanos(0),
            perceived_jitter: Duration::from_nanos(0),
            reported_rtt: Duration::from_nanos(0),
            reported_jitter: Duration::from_nanos(0),
        }
    }

    pub fn reset(&mut self) {
        self.last_delta = Duration::from_nanos(0);
        self.perceived_rtt = Duration::from_nanos(0);
        self.perceived_jitter = Duration::from_nanos(0);
        self.reported_rtt = Duration::from_nanos(0);
        self.reported_jitter = Duration::from_nanos(0);
    }

    pub fn msg(&self) -> MsgTimings {
        MsgTimings {
            sender_time: self.local_clock.elapsed().as_nanos() as u64,
            receiver_time: if self.last_delta.is_zero() {
                0
            } else {
                self.remote_clock.elapsed().as_nanos() as u64
            },
            perceived_rtt: self.perceived_rtt.as_nanos() as u64,
            perceived_jitter: self.perceived_jitter.as_nanos() as u64,
        }
    }

    pub fn update(&mut self, msg: &MsgTimings) {
        const R: f64 = Timings::IIR_RATIO;

        let now = self.local_clock.elapsed();
        let delta = now.abs_diff(Duration::from_nanos(msg.sender_time));

        dbg!(delta);

        if !self.last_delta.is_zero() {
            // Calculate the observed jitter
            let jitter = self.last_delta.abs_diff(delta);
            if self.perceived_jitter.is_zero() {
                // Set the perceived jitter to the observed jitter if it's the first measurement
                self.perceived_jitter = jitter;
            } else {
                // Otherwise use an IIR filter to smooth it
                self.perceived_jitter = self.perceived_jitter.mul_f64(R) + jitter.mul_f64(1.0 - R);
            }
        }

        if msg.receiver_time != 0 {
            // Calculate the observed RTT
            let rtt = now.abs_diff(Duration::from_nanos(msg.receiver_time));
            if self.perceived_rtt.is_zero() {
                // Set the perceived RTT to the observed RTT if it's the first measurement
                self.perceived_rtt = rtt;
            } else {
                // Otherwise use an IIR filter to smooth it
                self.perceived_rtt = self.perceived_rtt.mul_f64(R) + rtt.mul_f64(1.0 - R);
            }
        }

        self.remote_clock = Instant::now() + Duration::from_nanos(msg.sender_time);
        self.last_delta = delta;
        self.reported_rtt = Duration::from_nanos(msg.perceived_rtt);
        self.reported_jitter = Duration::from_nanos(msg.perceived_jitter);

        dbg!(self);
    }
}

#[derive(Debug, Clone)]
pub struct MsgTimings {
    /// The time of the sender's local clock, in nanoseconds.
    ///
    /// The clock offset is arbitrary, but constant so this value can be used to determine jitter between subsequent messages.
    pub sender_time: u64,
    /// The estimated time of the receiver's clock in nanoseconds.
    ///
    /// It is calcaluted as the sender timestamp of the last received message plus the time passed since then.
    /// The difference between this timestamp and the actual time of the receiver's clock on reception is the observed RTT.
    pub receiver_time: u64,
    pub perceived_rtt: u64,
    pub perceived_jitter: u64,
}

impl MsgTimings {
    pub fn encode(&self, buf: &mut [u8]) -> Option<usize> {
        buf[0] = 0x01;
        buf[1 + 0 * 8..][..8].copy_from_slice(&self.sender_time.to_be_bytes());
        buf[1 + 1 * 8..][..8].copy_from_slice(&self.receiver_time.to_be_bytes());
        buf[1 + 2 * 8..][..8].copy_from_slice(&self.perceived_rtt.to_be_bytes());
        buf[1 + 3 * 8..][..8].copy_from_slice(&self.perceived_jitter.to_be_bytes());
        Some(1 + 4 * 8)
    }

    pub fn decode(buf: &[u8]) -> Option<Self> {
        buf.first().filter(|x| **x == 0x01)?;
        Some(Self {
            sender_time: u64::from_be_bytes(buf.get(1 + 0 * 8..1 + 1 * 8)?.try_into().ok()?),
            receiver_time: u64::from_be_bytes(buf.get(1 + 1 * 8..1 + 2 * 8)?.try_into().ok()?),
            perceived_rtt: u64::from_be_bytes(buf.get(1 + 2 * 8..1 + 3 * 8)?.try_into().ok()?),
            perceived_jitter: u64::from_be_bytes(buf.get(1 + 3 * 8..1 + 4 * 8)?.try_into().ok()?),
        })
    }
}
