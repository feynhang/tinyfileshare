use std::{
    collections::HashMap,
    io::{BufRead, BufReader},
    net::{IpAddr, SocketAddr, TcpStream},
    path::PathBuf,
    str::FromStr,
};

use crossbeam::channel::Sender;
use uuid::Uuid;

use crate::{error::ServerError, handler::Handler, ServerResult};

#[derive(Debug, Clone, Copy)]
pub enum HeadCommand {
    Share,
    Send,
    Register,
}

impl HeadCommand {
    const SHARE_STR: &'static str = "SHARE";
    const TRANS_STR: &'static str = "TRANS";
    const REG_STR: &'static str = "REG";

    pub const fn to_static_str(&self) -> &'static str {
        match self {
            HeadCommand::Share => Self::SHARE_STR,
            HeadCommand::Send => Self::TRANS_STR,
            HeadCommand::Register => Self::REG_STR,
        }
    }
}

impl FromStr for HeadCommand {
    type Err = ServerError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let command = s.to_uppercase();
        match command.as_str() {
            Self::SHARE_STR => Ok(HeadCommand::Share),
            Self::TRANS_STR => Ok(HeadCommand::Send),
            Self::REG_STR => Ok(HeadCommand::Register),
            _ => Err(ServerError::InvalidRequest),
        }
    }
}

fn get_peer_server_addr() -> SocketAddr {
    todo!()
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
    pub(crate) fn dispatch(&mut self, stream: TcpStream) -> ServerResult<()> {
        let mut first_line = String::new();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        if reader.read_line(&mut first_line)? == 0 {
            return Err(crate::error::ServerError::InvalidRequest);
        }
        first_line = first_line.trim_end().to_owned();

        match HeadCommand::from_str(&first_line)? {
            HeadCommand::Share => {
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
                            raw_server_addr: get_peer_server_addr(),
                        })
                        .unwrap()
                }
            }
            HeadCommand::Send => self.handler_tx.send(Handler::RecvHandler(stream)).unwrap(),
            HeadCommand::Register => {
                let peer_addr = stream.peer_addr().unwrap();
                // 
                self.connected_hosts.insert(Uuid::new_v4(), peer_addr.ip());
            }
        }
        Ok(())
    }
}
