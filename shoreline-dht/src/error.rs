use std::fmt;
use crate::constants::*;

#[derive(Debug)]
pub enum Error {
    NodeTerminated,
    LinkTerminated,
    IdMissing,
    IdMismatch,
    InitTimeout,
    TotalTimeout,
    QueryTimeout,
    QueryError(i64, String),
    BencodeInvalid,
    ProtocolViolation,
    Socket(std::io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NodeTerminated => write!(f, "Node terminated"),
            Self::LinkTerminated => write!(f, "Link terminated"),
            Self::IdMissing => write!(f, "ID missing"),
            Self::IdMismatch => write!(f, "ID mismatch"),
            Self::InitTimeout => write!(f, "Init timed out after {}s", TIMEOUT_INIT.as_secs()),
            Self::QueryTimeout => write!(f, "Query timed out after {}x RTT", TIMEOUT_FACTOR),
            Self::TotalTimeout => write!(f, "Unresponsive for more than {}s", TIMEOUT_TOTAL.as_secs()),
            Self::BencodeInvalid => write!(f, "Received invalid bencode"),
            Self::ProtocolViolation => write!(f, "Protocol violation"),
            Self::QueryError(code, msg) => write!(f, "Received error code {}: {}", code, msg),
            Self::Socket(err) => write!(f, "{}", err),
        }
    }
}

impl std::error::Error for Error {}
