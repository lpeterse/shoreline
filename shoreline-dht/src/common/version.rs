use bencode_minimal::{TryFromValue, Value};
use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version([u8; 4]);

impl Version {
    pub const SELF: Self = Self([b'S', b'L', Self::SELF_MAJOR, Self::SELF_MINOR]);

    const SELF_MAJOR: u8 = env!("CARGO_PKG_VERSION_MAJOR").as_bytes()[0] - b'0';
    const SELF_MINOR: u8 = env!("CARGO_PKG_VERSION_MINOR").as_bytes()[0] - b'0';
}

impl AsRef<[u8]> for Version {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl TryFromValue<'_> for Version {
    fn try_from(v: &Value) -> Option<Self> {
        v.try_into::<[u8; 4]>().map(Self)
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let a = char::try_from(self.0[0]).unwrap_or('-');
        let b = char::try_from(self.0[1]).unwrap_or('-');
        let c = self.0[2];
        let d = self.0[3];
        if c.is_ascii_graphic() && d.is_ascii_graphic() {
            write!(f, "{}{}{}{}", a, b, c as char, d as char)
        } else {
            write!(f, "{}{}-{}.{}", a, b, c, d)
        }
    }
}
