use std::net::{IpAddr, SocketAddr};

use crate::global;

#[derive(Debug, Clone)]
pub(crate) struct Host {
    ip: IpAddr,
    count: u8,
}

impl PartialEq for Host {
    fn eq(&self, other: &Self) -> bool {
        self.ip == other.ip
    }
}

impl Eq for Host {}

impl Ord for Host {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.ip.cmp(&other.ip)
    }
}

impl PartialOrd for Host {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.ip.partial_cmp(&other.ip)
    }
}

impl Host {
    pub(crate) fn new(ip: IpAddr) -> Self {
        Self { ip, count: 0 }
    }

    pub(crate) fn increment_port(&mut self) {
        self.count += 1;
    }

    pub(crate) fn to_addr(&self) -> SocketAddr {
        SocketAddr::new(self.ip, global::config().port() + self.count as u16)
    }

    pub(crate) fn decrement_port(&mut self) {
        self.count = self.count.saturating_sub(1);
    }
}
