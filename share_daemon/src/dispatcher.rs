use std::{
    collections::HashMap,
    io::{BufRead, BufReader, BufWriter, Write},
    net::{IpAddr, SocketAddr, TcpStream},
    path::PathBuf,
    str::FromStr,
};

use crossbeam::channel::Sender;
use uuid::Uuid;

use crate::{config::User, error::CommonError, global, handler::Handler, CommonResult};

fn dest_server_addr() -> SocketAddr {
    unimplemented!()
}

#[derive(Debug, Clone)]
pub enum Command {
    Share(IpAddr),
    Send,
    Register(String),
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
            Command::Send => Self::SEND_STR.to_owned(),
            Command::Register(username) => format!("{} {}", Self::REG_STR, username),
            Command::Unsupported => "".to_owned(),
        };
        write!(f, "{}", s)
    }
}

impl<T: AsRef<str>> From<T> for Command {
    fn from(value: T) -> Self {
        let s = value.as_ref();
        if s == Self::SEND_STR {
            return Command::Send;
        }
        let res = s.split(' ').nth(1);
        if res.is_none() {
            return Command::Unsupported;
        }
        if s.starts_with(Self::REG_STR) {
            return Command::Register(res.unwrap().to_owned());
        }
        if s.starts_with(Self::SHARE_STR) {
            if let Ok(ip) = IpAddr::from_str(res.unwrap()) {
                return Command::Share(ip);
            }
        }
        Command::Unsupported
    }
}

pub struct Dispatcher {
    handler_tx: Sender<Handler>,
    connected_hosts: HashMap<Uuid, IpAddr>,
}

impl Dispatcher {
    pub(crate) fn new(handler_tx: Sender<Handler>) -> Self {
        Self {
            handler_tx,
            connected_hosts: HashMap::new(),
        }
    }

    fn log_invalid_request(addr: SocketAddr) {
        global::logger().log(
            format!("Invalid request from end point [{}], ignored it.", addr),
            crate::log::LogLevel::Warn,
        )
    }

    pub(crate) fn dispatch(&mut self, stream: TcpStream) -> CommonResult<()> {
        let mut first_line = String::new();
        let peer_addr = stream.peer_addr().unwrap();
        let mut reader = BufReader::new(stream.try_clone()?);
        let read_size = reader.read_line(&mut first_line)?;
        if read_size == 0 {
            Self::log_invalid_request(peer_addr);
            return Ok(());
        }
        first_line = first_line.trim_end().to_owned();

        match Command::from(&first_line) {
            Command::Share(addr) => {
                let mut paths = vec![PathBuf::from(&first_line)];
                let mut line = String::new();
                while reader.read_line(&mut line)? > 0 {
                    paths.push(PathBuf::from(line.trim_end()));
                    line.clear();
                }
                for path in paths {
                    self.handler_tx
                        .send(Handler::SendHandler {
                            path,
                            raw_server_addr: dest_server_addr(),
                        })
                        .unwrap()
                }
            }
            Command::Send => self.handler_tx.send(Handler::RecvHandler(stream)).unwrap(),
            Command::Register(name) => {
                if !global::config().check_user(&name) {
                    let mut writer = BufWriter::new(stream);
                    writer.write_all("ACCESS REFUSED\n".as_bytes())?;
                    return Ok(());
                }
                // let peer_addr = stream.peer_addr().unwrap();
                // validate client
                // if not valid, log and ignore request
                // if client is valid, insert
                self.connected_hosts.insert(Uuid::new_v4(), peer_addr.ip());
            }
            Command::Unsupported => Self::log_invalid_request(peer_addr),
        }
        Ok(())
    }
}
