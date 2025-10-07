use std::{net::IpAddr, path::Path};
use maxminddb::{Mmap, Reader, geoip2};

pub struct MMDB {
    db: Option<Reader<Mmap>>,
}

impl MMDB {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let db = maxminddb::Reader::open_mmap(&path).ok();
        if db.is_none() {
            log::warn!("Could not open MMDB at path: {}", path.as_ref().display());
        }
        Self { db }
    }

    pub fn lookup_iso<IP: Into<IpAddr>>(&self, ip: IP) -> Option<String> {
        let db = self.db.as_ref()?;
        let country = db.lookup::<geoip2::Country>(ip.into()).ok().flatten()?;
        country.country.and_then(|c| c.iso_code.map(|s| s.to_string()))
    }

    pub fn lookup_flag<IP: Into<IpAddr>>(&self, ip: IP) -> Option<String> {
        let iso = self.lookup_iso(ip.into())?;
        let bytes = iso.as_bytes();
        let (a,b) = (bytes.get(0)?, bytes.get(1)?);
        let mut s = String::new();
        s.push(char::from_u32(0x1F1E6 + (a - b'A') as u32)?);
        s.push(char::from_u32(0x1F1E6 + (b - b'A') as u32)?);
        Some(s)
    }
}
