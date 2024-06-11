use std::{
    // io::BufRead,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    str::FromStr,
};

use tokio::net::tcp::OwnedReadHalf;

use crate::{
    consts::{self, request},
    filedata::FileData,
    global,
    response::FailureResponse,
    server, CommonResult,
};

// const PORT_CONFIRM: &str = "PORT_CONFIRM";

#[derive(Debug, Clone)]
pub enum StartLine {
    TestReachable,
    Share(String),
    HostRegister { hostname: String, ip: IpAddr },
    PortPrepare(u16),
    Receive { file_name: String, file_size: usize },
}

impl StartLine {
    pub(crate) fn from_line_bytes(bytes: &[u8]) -> CommonResult<Self> {
        let invalid_ret = Err(crate::error::CommonError::FailureResp(
            FailureResponse::InvalidRequest,
        ));
        let mut parts = bytes.split(|b| *b == b' ');
        if parts.clone().count() == 2 {
            let cmd_bytes = parts.next().unwrap();
            if cmd_bytes.len() == 1 {
                let args = parts.next().unwrap();
                return match cmd_bytes[0] {
                    request::HOST_REG => {
                        todo!()
                    }
                    request::SHARE => {
                        todo!()
                    }
                    request::TEST_REACHABLE => Ok(Self::TestReachable),
                    _ => invalid_ret,
                };
            }
        }
        invalid_ret
    }
}

impl std::fmt::Display for StartLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use consts::request;
        match self {
            StartLine::Share(hostname) => write!(f, "{} {}", request::SHARE, hostname),
            StartLine::HostRegister { hostname, ip } => {
                write!(f, "{} {}:{}", request::HOST_REG, hostname, ip)
            }
            StartLine::Receive {
                file_name,
                file_size,
            } => write!(f, "{} {}:{}", request::RECV, file_name, file_size),
            StartLine::PortPrepare(port) => write!(f, "{} {}", request::PORT_PREPARE, port),
            StartLine::TestReachable => write!(f, "?"),
        }
    }
}

fn split_two_arg(arg: &str) -> Option<(&str, &str)> {
    let pair = arg.split(' ');
    if pair.clone().count() == 2 {
        let v: Vec<&str> = pair.collect();
        Some((v[0], v[1]))
    } else {
        None
    }
}

// impl StartLine {
//     pub async fn from_local(reader:&mut OwnedReadHalf) -> CommonResult<Self> {
//         let mut buf = vec![];
//         while let Ok(_) = reader.readable().await {
//             if reader.(&mut buf)? == 0 {
//                 break;
//             }
//         }
//         Ok(())
//     }
// }

// impl FromStr for StartLine {
//     type Err = FailureResponse;

//     fn from_str(s: &str) -> Result<Self, Self::Err> {
//         let mut start_line_itr = s.split(' ');
//         let parts_count = start_line_itr.clone().count();
//         match parts_count {
//             2 => {
//                 let arg = start_line_itr.nth(1).unwrap();
//                 match start_line_itr.nth(0).unwrap() {
//                     SHARE => return Ok(Self::Share(arg.to_owned())),
//                     RECV => {
//                         if let Some((name, value)) = split_two_arg(arg) {
//                             if let Ok(file_size) = usize::from_str_radix(value, 10) {
//                                 let file_name = name.to_owned();
//                                 return Ok(Self::Receive {
//                                     file_name,
//                                     file_size,
//                                 });
//                             }
//                         }
//                     }
//                     HOST_REG => {
//                         if let Some((name, value)) = split_two_arg(arg) {
//                             if let Ok(ip) = IpAddr::from_str(value) {
//                                 let hostname = name.to_owned();
//                                 return Ok(Self::HostRegister { hostname, ip });
//                             }
//                         }
//                     }
//                     PORT_PREPARE => {
//                         if let Ok(port) = u16::from_str_radix(arg, 10) {
//                             return Ok(Self::PortPrepare(port));
//                         }
//                     }
//                     _ => return Err(FailureResponse::InvalidRequest),
//                 }
//                 Err(FailureResponse::InvalidRequest)
//             }
//             1 => {
//                 if start_line_itr.next().unwrap() == TEST_REACHABLE {
//                     Ok(Self::TestReachable)
//                 } else {
//                     Err(FailureResponse::InvalidRequest)
//                 }
//             }
//             _ => Err(FailureResponse::InvalidRequest),
//         }
//     }
// }

#[derive(Debug, Clone)]
pub enum Host {
    Name(String),
    Addr(IpAddr),
}

impl Host {
    pub fn addr(&self) -> Option<IpAddr> {
        let config_store = server::config_store();
        match self {
            Host::Name(hostname) => config_store
                .get_ip_by_name(&hostname)
                .map(|addr_r| addr_r.clone()),
            Host::Addr(addr) => {
                if config_store.check_ip_registered(*addr) {
                    Some(*addr)
                } else {
                    None
                }
            }
        }
    }
}


#[derive(Debug)]
pub struct Request {
    // action
}

#[derive(Debug)]
pub enum LocalRequest {
    HostRegistration { hostname: String, ip: IpAddr },
    Share(Host),
}

#[derive(Debug)]
pub enum RemoteRequest {
    FileTransfer {
        name: String,
        size: usize,
        data: Vec<u8>,
    },
    TestReachable,
}
// impl Request {
//     pub fn from_stream(stream: &mut TcpStream) -> CommonResult<Self> {
//         let peer_addr = stream.peer_addr()?;
//         let mut line = String::new();
//         let mut reader = BufReader::with_capacity(30, stream).take(30);
//         if reader.read_line(&mut line)? != 0 {
//             let pair: Vec<String> = line.split(' ').map(|s| s.to_owned()).collect();
//             if pair.len() == 2 {
//                 let command_res = StartLine::from_str(&pair[0]);
//                 if command_res.is_ok() {
//                     line.clear();
//                     reader.set_limit(2);
//                     if reader.read_line(&mut line)? != 0 {
//                         if line.trim().is_empty() {
//                             line.clear();
//                             match command_res.unwrap() {
//                                 StartLine::Share => {
//                                     if peer_addr.ip().is_loopback() {
//                                         if let Some(hostname) = pair.get(1) {
//                                             let mut all_content = String::new();
//                                             let size = reader
//                                                 .into_inner()
//                                                 .read_to_string(&mut all_content)?;
//                                             if size != 0 && !all_content.trim().is_empty() {
//                                                 let mut lines = all_content.split("\r\n");
//                                                 let mut files_paths = vec![];
//                                                 while let Some(l) = lines.next() {
//                                                     files_paths
//                                                         .push(PathBuf::from_str(l.trim()).unwrap());
//                                                 }
//                                                 if !files_paths.is_empty() {
//                                                     return Ok(Self::FileShare {
//                                                         hostname: hostname.to_string(),
//                                                         files_paths,
//                                                     });
//                                                 }
//                                             }
//                                         }
//                                     }
//                                 }
//                                 StartLine::HostRegister => {
//                                     if peer_addr.ip().is_loopback() {
//                                         let host_pair_str = pair[1].clone();
//                                         let host_pair: Vec<&str> =
//                                             host_pair_str.split(':').collect();
//                                         if host_pair.len() == 2 {
//                                             if let Ok(ip) = IpAddr::from_str(host_pair[1]) {
//                                                 return Ok(Self::HostRegister(
//                                                     host_pair[0].to_owned(),
//                                                     ip,
//                                                 ));
//                                             }
//                                         }
//                                     }
//                                 }
//                                 StartLine::Receive => {
//                                     if global::config().check_ip_registered(peer_addr.ip()) {

//                                     }
//                                 }
//                                 StartLine::PortPrepare => todo!(),
//                             }
//                         }
//                     }
//                 }
//             }
//         }
//         Err(CommonError::FailureResp(
//             crate::response::FailureResponse::InvalidRequest,
//         ))
//     }
// }

#[cfg(test)]
mod tests {
    use std::io::{BufRead, Cursor, Read};

    #[test]
    fn take_test() {
        let cursor = Cursor::new(
            r#"
smdfangoenad,mfne
;snmalgk;
as;dioweop23904
34h34opwefklasqw
32945naldf23
94bnsl94
"#,
        );
        let mut reader = cursor.take(4);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        assert_eq!(line, "\n");
        line.clear();
        reader.set_limit(4);
        reader.read_line(&mut line).unwrap();
        assert_eq!(line.as_bytes(), "smdf".as_bytes());
    }

    #[test]
    fn size_hint_test() {
        let s = "REG myhost:1.1.1.1";
        let pair = s.split(' ');
        let pair_size = pair.clone().count();
        assert_eq!(2_usize, pair_size);
    }
}
