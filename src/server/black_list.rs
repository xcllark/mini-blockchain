use std::net::SocketAddr;

#[derive(Debug, Clone, Default)]
pub struct BlackList {
    blacklisted: Vec<SocketAddr>,
}

impl BlackList {
    pub fn contains(&self, ip: &SocketAddr) -> bool {
        self.blacklisted.contains(ip)    
    }
}
