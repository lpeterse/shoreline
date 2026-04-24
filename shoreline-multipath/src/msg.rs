use std::net::{Ipv6Addr, SocketAddrV6};

#[derive(Debug, Clone)]
pub struct MsgAddresses {
    addrs: Vec<Ipv6Addr>,
}

impl MsgAddresses {
    pub fn encode(&self, buf: &mut [u8]) -> Option<usize> {
        buf[0] = 0x02;
        buf[1] = self.addrs.len() as u8;
        for (i, addr) in self.addrs.iter().enumerate() {
            let offset = 1 + 1 + i * 16;
            if offset + 16 > buf.len() {
                return None;
            }
            buf[offset..offset + 16].copy_from_slice(&addr.octets());
        }
        Some(1 + 1 + self.addrs.len() * 16)
    }

    pub fn decode(buf: &[u8]) -> Option<Self> {
        buf.first().filter(|x| **x == 0x02)?;
        let len = *(buf.get(1)?) as usize;
        let mut addrs = Vec::with_capacity(len);
        for i in 0..len {
            let offset = 1 + 1 + i * 16;
            let ip = buf.get(offset..offset + 16)?.try_into().ok()?;
            let ip = Ipv6Addr::from_octets(ip);
            addrs.push(ip);
        }
        Some(Self { addrs })
    }
}
