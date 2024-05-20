use std::net::SocketAddr;


pub struct SvrConfig {
    addr: SocketAddr,
    conf_file_path: String
}

impl Default for SvrConfig {
    fn default() -> Self {
        Self { addr: SocketAddr::new(crate::LOCALHOST, 2171), conf_file_path: crate::get_home_path() }
    }
}