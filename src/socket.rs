use std::net::SocketAddrV6;
use tokio::sync::{mpsc, watch};
use tokio::time::sleep;
use tokio::time::{Duration, Instant};
use tokio::{net::UdpSocket, select};
use tokio_util::sync::CancellationToken;

pub struct Socket {}

impl Socket {
    pub fn send(&self, data: &[u8]) -> Result<(), crate::Error> {
        Ok(())
    }

    pub fn receive(&self) -> Result<Vec<u8>, crate::Error> {
        Ok(vec![])
    }
}

pub struct SocketTask {
    addrs_local: watch::Receiver<Vec<SocketAddrV6>>,
    addrs_remote: watch::Receiver<Vec<SocketAddrV6>>,
    paths: Vec<Path>,
    token: CancellationToken,
}

impl SocketTask {
    pub fn new(
        addrs_local: watch::Receiver<Vec<SocketAddrV6>>,
        addrs_remote: watch::Receiver<Vec<SocketAddrV6>>,
    ) -> Self {
        SocketTask { addrs_local, addrs_remote, paths: Vec::new(), token: CancellationToken::new() }
    }

    pub async fn run(mut self) {
        loop {
            select! {
                r = self.addrs_local.changed() => {
                    match r {
                        Ok(_) => self.update_paths(),
                        Err(_) => break,
                    }
                }
                r = self.addrs_remote.changed() => {
                    match r {
                        Ok(_) => self.update_paths(),
                        Err(_) => break,
                    }
                }
            }
        }
    }

    fn update_paths(&mut self) {
        let locals = { self.addrs_local.borrow().clone() };
        let remotes = { self.addrs_remote.borrow().clone() };
        // remove paths that are no longer valid
        self.paths.retain(|path| locals.contains(&path.addr_local) && remotes.contains(&path.addr_remote));
        // add new paths for new local and remote addresses
        for local in &locals {
            for remote in &remotes {
                if !self.paths.iter().any(|path| path.addr_local == *local && path.addr_remote == *remote) {
                    let path = Path::new(*local, *remote, self.token.child_token());
                    self.paths.push(path);
                }
            }
        }
    }
}

pub struct Path {
    addr_local: SocketAddrV6,
    addr_remote: SocketAddrV6,
    token: CancellationToken,
}

impl Path {
    pub fn new(addr_local: SocketAddrV6, addr_remote: SocketAddrV6, token: CancellationToken) -> Self {
        let task = PathTask::new(addr_local, addr_remote);
        let _ = tokio::spawn(task.run(token.clone()));
        Path { addr_local, addr_remote, token }
    }
}

impl Drop for Path {
    fn drop(&mut self) {
        self.token.cancel();
    }
}

pub struct PathTask {
    addr_local: SocketAddrV6,
    addr_remote: SocketAddrV6,
    timings: Timings,
    rbuf: Vec<u8>,
    upstream: mpsc::UnboundedSender<Vec<u8>>,
    socket: Result<UdpSocket, std::io::Error>,
}

impl PathTask {
    pub fn new(addr_local: SocketAddrV6, addr_remote: SocketAddrV6) -> Self {
        let socket = socket_connected(&addr_local, &addr_remote);
        let rbuf = vec![0u8; 1500];
        let (upstream, _) = mpsc::unbounded_channel();
        PathTask {
            addr_local,
            addr_remote,
            timings: Timings::default(),
            rbuf,
            upstream,
            socket,
        }
    }

    pub async fn run(mut self, token: CancellationToken) {
        loop {
            select! {
                _ = token.cancelled() => break,
                _ = sleep(Duration::from_secs(30)) => self.send_ping().await,
                _ = self.receive() => {},
            }
        }
    }

    async fn receive(&mut self) {
        if let Ok(socket) = &self.socket {
            if let Ok(rcvd) = socket.recv(&mut self.rbuf).await {
                let buf = &self.rbuf[..rcvd];
                if let Some(msg) = MsgTimings::decode(buf) {
                    self.timings.update(&msg);
                }
            }
        } else {
            std::future::pending().await
        }
    }

    async fn send_ping(&mut self) {
        if self.socket.is_err() {
            self.socket = socket_connected(&self.addr_local, &self.addr_remote);
        }
        if let Ok(socket) = &self.socket {
            let mut buf = [0u8; 1 + 4*8];
            let msg = self.timings.msg();
            let _ = msg.encode(&mut buf);
            if let Err(e) = socket.send(&buf).await {
                self.socket = Err(e);
            }
        }
    }
}

pub struct Timings {
    local_clock: Instant,
    remote_clock: Instant,
    last_delta: Duration,
    perceived_rtt: Duration,
    perceived_jitter: Duration,
    reported_rtt: Duration,
    reported_jitter: Duration,
}

impl Timings {
    const IIR_RATIO: f64 = 0.9;

    fn msg(&self) -> MsgTimings {
        MsgTimings {
            sender_time: self.local_clock.elapsed().as_nanos() as u64,
            receiver_time: self.remote_clock.elapsed().as_nanos() as u64,
            perceived_rtt: self.perceived_rtt.as_nanos() as u64,
            perceived_jitter: self.perceived_jitter.as_nanos() as u64,
        }
    }

    fn update(&mut self, msg: &MsgTimings) {
        const R: f64 = Timings::IIR_RATIO;
        let now = self.local_clock.elapsed();
        // Calculate the observed jitter
        let delta = now.abs_diff(Duration::from_nanos(msg.sender_time));
        let jitter = self.last_delta.abs_diff(delta);
        // Calculate the observed RTT
        let rtt = now.abs_diff(Duration::from_nanos(msg.receiver_time));
        // Update the the fields
        self.remote_clock = Instant::now() + Duration::from_nanos(msg.sender_time);
        self.last_delta = delta;
        self.perceived_rtt = self.perceived_rtt.mul_f64(R) + rtt.mul_f64(1.0 - R);
        self.perceived_jitter = self.perceived_jitter.mul_f64(R) + jitter.mul_f64(1.0 - R);
        self.reported_rtt = Duration::from_nanos(msg.perceived_rtt);
        self.reported_jitter = Duration::from_nanos(msg.perceived_jitter);
    }
}

impl Default for Timings {
    fn default() -> Self {
        Self { local_clock: Instant::now(), remote_clock: Instant::now(), last_delta: Duration::from_nanos(0), perceived_jitter: Duration::from_nanos(0), perceived_rtt: Duration::from_nanos(0), reported_rtt: Duration::from_nanos(0), reported_jitter: Duration::from_nanos(0) }
    }
}

pub struct MsgTimings {
    /// The time of the sender's local clock, in nanoseconds.
    ///
    /// The clock offset is arbitrary, but constant so this value can be used to determined jitter on receiving a message.
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

fn socket_connected(bind: &SocketAddrV6, conn: &SocketAddrV6) -> Result<UdpSocket, std::io::Error> {
    use socket2::{Domain, Protocol, Socket, Type};
    let socket = Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_only_v6(true)?;
    socket.set_reuse_address(true)?;
    socket.set_reuse_port(true)?;
    socket.bind(&(*bind).into())?;
    socket.connect(&(*conn).into())?;
    socket.set_nonblocking(true)?;
    Ok(UdpSocket::from_std(socket.into())?)
}
