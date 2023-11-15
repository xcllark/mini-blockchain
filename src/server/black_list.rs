use std::net::SocketAddr;


#[derive(Debug, Clone, Default)]
pub struct BlackList {
    blacklisted: Vec<SocketAddr>,
}

impl BlackList {
    pub fn contains(&self, ip: &SocketAddr) -> bool {
        if self.blacklisted.contains(ip) {
            true
        } else {
            false
        }
    }
}
