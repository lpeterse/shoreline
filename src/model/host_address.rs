use std::{ops::Deref, str::FromStr};

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde_with::DeserializeFromStr, serde_with::SerializeDisplay)]
pub struct HostAddress(String);

impl HostAddress {
    pub const EXAMPLE: &'static str = "example.com:6881";

    pub async fn resolve(&self) -> Result<std::net::SocketAddrV6, String> {
        let mut addrs = tokio::net::lookup_host(&self.0).await.map_err(|e| format!("DNS resolution failed: {}", e))?;
        addrs
            .find_map(|a| if let std::net::SocketAddr::V6(a) = a { Some(a) } else { None })
            .ok_or_else(|| "No valid IPv6 address found".to_string())
    }
}

impl Deref for HostAddress {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromStr for HostAddress {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (host, port) = s.split_once(':').ok_or_else(|| "Invalid format, expected host:port".to_string())?;
        if host.split('.').any(|s| s.is_empty() || !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')) {
            return Err("Invalid host format".to_string());
        }
        port.parse::<u16>().map_err(|_| "Invalid port number".to_string())?;
        Ok(Self(s.to_string()))
    }
}

impl std::fmt::Display for HostAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
