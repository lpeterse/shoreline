use std::fmt;

#[derive(Debug)]
pub enum Error {
    NodeDropped,
    NodeTerminated,
    PeerDropped,
    PeerTerminated,
    PeerIdMissing,
    PeerIdMismatch,
    PeerInitTimeout,
    PeerNotConnected,
    PeerQueryTimeout,
    PeerQueryError(i64, String),
    PeerBencodeInvalid,
    PeerProtocolViolation,
    Socket(std::io::Error),
    Other(Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NodeDropped => write!(f, "Node dropped"),
            Self::NodeTerminated => write!(f, "Node terminated"),
            Self::PeerDropped => write!(f, "Peer dropped"),
            Self::PeerTerminated => write!(f, "Peer terminated"),
            Self::PeerIdMissing => write!(f, "Peer ID missing"),
            Self::PeerIdMismatch => write!(f, "Peer ID mismatch"),
            Self::PeerInitTimeout => write!(f, "Peer init timed out"),
            Self::PeerNotConnected => write!(f, "Peer not connected"),
            Self::PeerBencodeInvalid => write!(f, "Peer sent invalid bencode"),
            Self::PeerProtocolViolation => write!(f, "Peer protocol violation"),
            Self::PeerQueryError(code, msg) => write!(f, "Peer sent error code {}: {}", code, msg),
            Self::PeerQueryTimeout => write!(f, "Peer command timed out"),
            Self::Socket(err) => write!(f, "Socket: {}", err),
            Self::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

impl From<&'static str> for Error {
    fn from(msg: &'static str) -> Self {
        Self::Other(msg.into())
    }
}

impl From<String> for Error {
    fn from(msg: String) -> Self {
        Self::Other(msg.into())
    }
}
