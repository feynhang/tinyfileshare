use std::{
    io::{BufRead, BufReader},
    net::{IpAddr, SocketAddr, TcpStream},
    path::PathBuf,
    str::FromStr,
};

use crossbeam::channel::Sender;

use crate::{
    error::CommonError, global, handler::Handler, host::Host, response::ResponseCode, CommonResult,
};

#[derive(Debug, Clone)]
pub enum Command {
    Share(IpAddr),
    Send,
    Register(IpAddr),
    Unsupported,
}

impl Command {
    const SHARE_STR: &'static str = "SHARE";
    const SEND_STR: &'static str = "SEND";
    const REG_STR: &'static str = "REG";
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Command::Share(addr) => format!("{} {}", Self::SHARE_STR, addr),
            Command::Send=> format!("{}", Self::SEND_STR),
            Command::Register(username) => format!("{} {}", Self::REG_STR, username),
            Command::Unsupported => "".to_owned(),
        };
        write!(f, "{}", s)
    }
}

impl<T: AsRef<str>> From<T> for Command {
    fn from(value: T) -> Self {
        let parts: Vec<&str> = value.as_ref().split(' ').map(|s| s.trim()).collect();
        if parts.len() != 2 {
            return Command::Unsupported;
        }
        match parts[0] {
            Self::SEND_STR => {
                return Command::Send;
            }
            Self::REG_STR => {
                if let Ok(ip) = IpAddr::from_str(parts[1]) {
                    return Command::Register(ip);
                }

                Command::Unsupported
            }
            Self::SHARE_STR => {
                if let Ok(ip) = IpAddr::from_str(parts[1]) {
                    return Command::Share(ip);
                }
                Command::Unsupported
            }
            _ => Command::Unsupported,
        }
        // if s == Self::SEND_STR {
        //     return Command::Send;
        // }
        // let res = ;
        // if res.is_none() {
        //     return Command::Unsupported;
        // }
        // if s.starts_with(Self::REG_STR) {
        //     return Command::Register(res.unwrap().to_owned());
        // }
        // if s.starts_with(Self::SHARE_STR) {
        //     if let Ok(ip) = IpAddr::from_str(res.unwrap()) {
        //         return Command::Share(ip);
        //     }
        // }
        // Command::Unsupported
    }
}

pub struct Dispatcher {
    handler_tx: Sender<Handler>,
}

impl Dispatcher {
    pub(crate) fn new(handler_tx: Sender<Handler>) -> Self {
        Self { handler_tx }
    }

    // fn log_invalid_request(addr: SocketAddr) {
    //     global::logger().log(
    //         format!("Invalid request from end point [{}], ignored it.", addr),
    //         crate::log::LogLevel::Warn,
    //     )
    // }

    fn check_localhost(&mut self, ip: IpAddr, conn: TcpStream) -> Option<TcpStream> {
        if !ip.is_loopback() {
            self.handler_tx
                .send(Handler::MsgSendHandler(conn, ResponseCode::InvalidRequest))
                .unwrap();
            return None;
        }
        return Some(conn);
    }

    pub(crate) fn dispatch(
        &mut self,
        connection: TcpStream,
        peer_addr: SocketAddr,
    ) -> CommonResult<()> {
        let mut first_line = String::new();
        let mut reader = BufReader::new(connection.try_clone()?);
        let read_size = reader.read_line(&mut first_line)?;
        if read_size == 0 {
            self.handler_tx
                .send(Handler::MsgSendHandler(
                    connection,
                    ResponseCode::InvalidRequest,
                ))
                .unwrap();
            return Ok(());
        }
        first_line = first_line.trim_end().to_owned();

        match Command::from(&first_line) {
            Command::Share(target_ip) => {
                let conn_opt = self.check_localhost(peer_addr.ip(), connection);
                if conn_opt.is_none() {
                    return Ok(());
                }
                let find_res = global::registered_hosts().binary_search(&Host::new(target_ip));
                if find_res.is_err() {
                    self.handler_tx
                        .send(Handler::MsgSendHandler(
                            conn_opt.unwrap(),
                            ResponseCode::UnRegistered,
                        ))
                        .unwrap();
                    return Ok(());
                }
                let mut paths = vec![PathBuf::from(&first_line)];
                let mut line = String::new();
                while reader.read_line(&mut line)? > 0 && line.trim().len() > 0 {
                    paths.push(PathBuf::from(line.trim()));
                    line.clear();
                }
                self.handler_tx
                    .send(Handler::FileSendHandler {
                        conn: conn_opt.unwrap(),
                        paths,
                        host_reg_index: find_res.unwrap(),
                    })
                    .unwrap()
            }
            Command::Send => self
                .handler_tx
                .send(Handler::RecvHandler(connection))
                .unwrap(),
            Command::Register(target_ip) => {
                let conn_opt = self.check_localhost(peer_addr.ip(), connection);
                if conn_opt.is_none() {
                    return Ok(());
                }
                self.handler_tx
                    .send(Handler::RegisterHandler(
                        conn_opt.unwrap(),
                        Host::new(target_ip),
                    ))
                    .unwrap();
            }
            Command::Unsupported => self
                .handler_tx
                .send(Handler::MsgSendHandler(
                    connection,
                    ResponseCode::InvalidRequest,
                ))
                .unwrap(),
        }
        Ok(())
    }
}
