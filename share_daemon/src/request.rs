use std::{
    collections::HashMap,
    io::{BufReader, Read},
    net::TcpStream,
    str::FromStr,
};

use crate::{
    error::CommonError,
    response::Response,
    util::BytesExtensions,
};

const RECV: &str = "RECV";
const SHARE: &str = "SHARE";
const REG: &str = "REG";

#[derive(Debug, Clone, Copy)]
pub enum RequestCommand {
    FileShare,
    HostRegistration,
    FileReceive,
}

#[derive(Debug)]
pub struct Request {
    command: RequestCommand,
    headers: HashMap<String, String>,
    data: Option<Vec<u8>>,
}

impl Request {
    pub fn kind(&self) -> RequestCommand {
        self.command
    }
}

impl TryFrom<TcpStream> for Request {
    type Error = CommonError;

    fn try_from(stream: TcpStream) -> Result<Self, Self::Error> {
        static mut CURR_IDX: usize = 0;

        fn next_line(bytes: &[u8]) -> Option<&[u8]> {
            unsafe {
                if CURR_IDX < bytes.len() {
                    if let Some(newline_idx) = bytes[CURR_IDX..].find_first(b'\n') {
                        let ret = Some(&bytes[CURR_IDX..=newline_idx]);
                        CURR_IDX = newline_idx + 1;

                        return ret;
                    }
                }
            }
            None
        }

        let mut buf = vec![];
        let mut reader = BufReader::new(stream);
        reader.read_to_end(&mut buf)?;
        let bytes = buf.trim();
        if let Some(cmd_bytes) = bytes.nth_line(0) {
            if let Ok(cmd_str) = std::str::from_utf8(cmd_bytes) {
                let command = RequestCommand::from_str(cmd_str)?;

                if let Some(remain_bytes) = bytes.skip_lines(1) {
                    if let Some(empty_line) = remain_bytes.nth_line(0) {
                        if empty_line.trim().is_empty() {
                            let mut headers: HashMap<String, String> = HashMap::new();

                            while let Some(line) = next_line(remain_bytes) {
                                let header_bytes = line.trim();
                                if header_bytes.is_empty() {
                                    break;
                                }
                                let header_pair: Vec<&[u8]> =
                                    header_bytes.split(|b| *b == b':').collect();
                                if header_pair.len() != 2 {
                                    continue;
                                }
                                let name_res = std::str::from_utf8(header_pair[0]);
                                let value_res = std::str::from_utf8(header_pair[1]);
                                if name_res.is_err() || value_res.is_err() {
                                    continue;
                                }
                                headers.insert(
                                    name_res.unwrap().to_owned(),
                                    value_res.unwrap().to_owned(),
                                );
                            }
                            let curr_idx = unsafe { CURR_IDX };
                            let data = if remain_bytes.get(curr_idx).is_some() {
                                Some(remain_bytes[curr_idx..].trim().to_vec())
                            } else {
                                None
                            };
                            return Ok(Self {
                                command,
                                headers,
                                data,
                            });
                        }
                    }
                }
            }
        }

        Err(CommonError::ReplyErr(Response::InvalidRequest))
    }
}

impl AsRef<str> for RequestCommand {
    fn as_ref(&self) -> &str {
        match self {
            RequestCommand::FileShare => SHARE,
            RequestCommand::HostRegistration => REG,
            RequestCommand::FileReceive => RECV,
        }
    }
}

impl FromStr for RequestCommand {
    type Err = CommonError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            RECV => Ok(Self::FileReceive),
            SHARE => Ok(Self::FileShare),
            REG => Ok(Self::HostRegistration),
            _ => Err(CommonError::ReplyErr(Response::InvalidRequest)),
        }
    }
}
