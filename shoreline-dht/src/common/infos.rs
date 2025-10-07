use std::{net::{Ipv6Addr, SocketAddrV6}, ops::{Deref, DerefMut}};
use crate::util::check;
use super::{Id, Info};

#[derive(Clone, Debug, Default)]
pub struct Infos(Vec<Info>);

impl Infos {
    pub fn new() -> Self {
        Self(Vec::with_capacity(8))
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(38 * self.len());
        for info in self.iter() {
            v.extend_from_slice(info.id.as_ref());
            v.extend_from_slice(&info.addr.ip().octets());
            v.extend_from_slice(&info.addr.port().to_be_bytes());
        }
        v
    }

    pub fn decode(buf: &[u8]) -> Option<Self> {
        check(buf.len() % 38 == 0)?;
        let mut v = Vec::with_capacity(buf.len() / 38);
        for chunk in buf.chunks(38) {
            let id: [u8; 20] = chunk[0..20].try_into().ok()?;
            let ip: [u8; 16] = chunk[20..36].try_into().ok()?;
            let pt: [u8; 2] = chunk[36..38].try_into().ok()?;
            let id = Id::from_bytes(&id);
            let ip = Ipv6Addr::from(ip);
            let pt = u16::from_be_bytes(pt);
            v.push(Info::new(id, SocketAddrV6::new(ip, pt, 0, 0)));
        }
        Some(Self(v))
    }
}

impl From<Vec<Info>> for Infos {
    fn from(v: Vec<Info>) -> Self {
        Self(v)
    }
}

impl Into<Vec<Info>> for Infos {
    fn into(self) -> Vec<Info> {
        self.0
    }
}

impl Deref for Infos {
    type Target = Vec<Info>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Infos {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// impl Encodable for NodeInfos {
//     fn encode(&self, enc: &mut Encoder) -> Option<()> {
//         enc.push_usize(38 * self.len())?;
//         enc.push_u8(b':')?;
//         for info in self.iter() {
//             enc.push_raw(info.id.as_ref())?;
//             enc.push_raw(&info.addr.ip().octets())?;
//             enc.push_raw(&info.addr.port().to_be_bytes())?;
//         }
//         Some(())
//     }
// }
