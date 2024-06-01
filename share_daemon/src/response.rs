use std::net::IpAddr;


#[derive(Debug)]
pub enum Response {
    InvalidRequest,
    UnregisteredHost,
    RegisterFailed(String),
    FileSendProgress(f64),
    FileIoErr(std::io::Error),
    ConnectHostFailed(String, IpAddr, std::io::Error),
}

impl std::fmt::Display for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}