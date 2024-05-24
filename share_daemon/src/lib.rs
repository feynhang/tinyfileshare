use std::net::{IpAddr, Ipv4Addr};

use config::SvrConfig;
use svr_err::ServerError;


#[allow(unused)]
pub mod config;
pub mod svr_err;
pub const LOCALHOST: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

#[cfg(windows)]
fn get_home_path() -> String {
    std::env::var("USERPROFILE").unwrap()
}

#[cfg(not(windows))]
fn get_home_path() -> String {
    std::env::var("HOME").unwrap()
}

pub fn port() -> u16 {
    todo!()
}

pub fn set_port() {
    todo!()
}

pub fn start_service(_config: Option<SvrConfig>) -> Result<(), ServerError> {
    todo!()
}






#[cfg(test)]
mod tests {
    use crate::get_home_path;

    #[test]
    fn test_get_home() {
        assert_eq!("C:\\Users\\feyn", get_home_path())
    }
}