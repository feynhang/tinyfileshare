use std::net::SocketAddr;

use smol_str::{SmolStr, ToSmolStr};

use crate::consts;

#[derive(Debug, Clone)]
pub enum RequestCommand {
    Local(LocalCommand),
    PortCheck,
}

impl RequestCommand {
    const PORT_CHECK: &'static str = "PORT";
    pub fn as_str(&self) -> &str {
        match self {
            RequestCommand::Local(c) => c.as_str(),
            RequestCommand::PortCheck => Self::PORT_CHECK,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum LocalCommand {
    Share,
    Register,
}

impl LocalCommand {
    const SHARE: &'static str = "SHARE";
    const REG: &'static str = "REG";

    pub fn as_bytes(&self) -> &[u8] {
        self.as_str().as_bytes()
    }

    pub fn as_str(&self) -> &str {
        match self {
            LocalCommand::Share => Self::SHARE,
            LocalCommand::Register => Self::REG,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Response {
    InvalidHostname,
    RegisterSucceeded,
    UnexpectedResponse,
    Remote(RemoteResponse),
    Local(LocalResponse),
}

impl Response {
    const REG_SUCCEEDED: &'static str = "REG_SUCCEEDED";
    const UNEXPECTED_RESP: &'static str = "UNEXPECTED_RESPONSE";
    const INVALID_HOSTNAME: &'static str = "INVALID_HOSTNAME";
}

impl ToSmolStr for Response {
    fn to_smolstr(&self) -> SmolStr {
        match self {
            Response::InvalidHostname => Self::INVALID_HOSTNAME.into(),
            Response::RegisterSucceeded => Self::REG_SUCCEEDED.into(),
            Response::UnexpectedResponse => Self::UNEXPECTED_RESP.into(),
            Response::Remote(r_resp) => r_resp.to_smolstr(),
            Response::Local(l_resp) => l_resp.to_smolstr(),
        }
    }
}

impl Response {
    pub fn to_str_unchecked(self) -> &'static str {
        match self {
            Response::InvalidHostname => Self::INVALID_HOSTNAME,
            Response::RegisterSucceeded => Self::REG_SUCCEEDED,
            Response::UnexpectedResponse => Self::UNEXPECTED_RESP,
            Response::Remote(r) => r.to_str_unchecked(),
            Response::Local(l) => l.to_str_unchecked(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum RemoteResponse {
    UnregisteredHost,
    NoAvailablePort,
    PortConfirm(u16),
    InvalidPort,
    FilesReceived(u8),
    UnexpectedEndFlag(u8),
    InvalidRequest,
}

impl RemoteResponse {
    const UNREGISTERED_HOST: &'static str = "UNREG_HOST";
    const NO_AVAILABLE_PORT: &'static str = "NO_AVAILABLE_PORT";
    const PORT_CONFIRM: &'static str = "PORT_CONFIRM";
    const INVALID_PORT: &'static str = "INVALID_PORT";
    const FILES_RECEIVED: &'static str = "FILES_RECEIVED";
    const UNEXPECTED_END_FLAG: &'static str = "UNEXPECTED_END_FLAG";
    const INVALID_REQUEST: &'static str = "INVALID_REQUEST";
}

impl std::str::FromStr for RemoteResponse {
    type Err = Response;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(Response::UnexpectedResponse);
        }
        let mut maybe_pair = s.trim().split(consts::STARTLINE_SEP);
        match unsafe { maybe_pair.next().unwrap_unchecked() } {
            Self::PORT_CONFIRM => {
                if let Some(port_str) = maybe_pair.next() {
                    if let Ok(p) = port_str.parse::<u16>() {
                        return Ok(Self::PortConfirm(p));
                    }
                }
                Err(Response::UnexpectedResponse)
            }
            Self::FILES_RECEIVED => {
                if let Some(count_str) = maybe_pair.next() {
                    if let Ok(count) = count_str.parse::<u8>() {
                        return Ok(Self::FilesReceived(count));
                    }
                }
                Err(Response::UnexpectedResponse)
            }
            Self::UNEXPECTED_END_FLAG => {
                if let Some(count_str) = maybe_pair.next() {
                    if let Ok(count) = count_str.parse::<u8>() {
                        return Ok(Self::UnexpectedEndFlag(count));
                    }
                }
                Err(Response::UnexpectedResponse)
            }
            Self::NO_AVAILABLE_PORT => Ok(Self::NoAvailablePort),
            Self::UNREGISTERED_HOST => Ok(Self::UnregisteredHost),
            Self::INVALID_PORT => Ok(Self::InvalidPort),
            Self::INVALID_REQUEST => Ok(Self::InvalidRequest),
            _ => Err(Response::UnexpectedResponse),
        }
    }
}

impl ToSmolStr for RemoteResponse {
    fn to_smolstr(&self) -> SmolStr {
        match self {
            RemoteResponse::UnregisteredHost => Self::UNREGISTERED_HOST.to_smolstr(),
            RemoteResponse::NoAvailablePort => Self::NO_AVAILABLE_PORT.to_smolstr(),
            RemoteResponse::PortConfirm(p) => {
                smol_str::format_smolstr!("{} {}", Self::PORT_CONFIRM, p)
            }
            RemoteResponse::InvalidPort => Self::INVALID_PORT.to_smolstr(),
            RemoteResponse::FilesReceived(n) => {
                smol_str::format_smolstr!("{} {}", Self::FILES_RECEIVED, *n)
            }
            RemoteResponse::UnexpectedEndFlag(count) => {
                smol_str::format_smolstr!("{} {}", Self::UNEXPECTED_END_FLAG, *count)
            }
            RemoteResponse::InvalidRequest => Self::INVALID_REQUEST.to_smolstr(),
        }
    }
}

impl RemoteResponse {
    pub fn to_str_unchecked(self) -> &'static str {
        match self {
            RemoteResponse::UnregisteredHost => Self::UNREGISTERED_HOST,
            RemoteResponse::NoAvailablePort => Self::NO_AVAILABLE_PORT,
            RemoteResponse::InvalidPort => Self::INVALID_PORT,
            RemoteResponse::InvalidRequest => Self::INVALID_REQUEST,
            _ => "",
        }
    }
}

#[derive(Debug, Clone)]
pub enum LocalResponse {
    RemoteUnregistered,
    RemoteNoAvailablePort,
    UnreachableAddress(SocketAddr),
    AllFilesSucceeded,
    FileInfo(SmolStr, Option<u64>),
    Progress(f64),
    FilesSucceeded(u8),
    LocalRegisterFailed,
    UnexpectedSendResp,
    ReplacedAddress(SocketAddr),
    UnregisteredHostname,
    AnyPathInvalid,
    UnexpectedRemoteResponse,
}

impl ToSmolStr for LocalResponse {
    fn to_smolstr(&self) -> SmolStr {
        match self {
            LocalResponse::RemoteUnregistered => Self::R_UNREG_HOST.to_smolstr(),
            LocalResponse::RemoteNoAvailablePort => Self::R_NO_AVAILABLE_PORT.to_smolstr(),
            LocalResponse::UnreachableAddress(a) => {
                smol_str::format_smolstr!("{} {}", Self::UNREACHABLE, a)
            }
            LocalResponse::AllFilesSucceeded => Self::ALL_FILES_SUCCEEDED.to_smolstr(),
            LocalResponse::FileInfo(name, size) => smol_str::format_smolstr!(
                "{} {}:{}",
                Self::FILE_INFO,
                name,
                size.map(|u| u as i64).unwrap_or(-1)
            ),
            LocalResponse::Progress(p) => smol_str::format_smolstr!("{} {}", Self::PROGRESS, *p),
            LocalResponse::FilesSucceeded(files_count) => {
                smol_str::format_smolstr!("{} {}", Self::FILES_SUCCEEDED, *files_count)
            }
            LocalResponse::LocalRegisterFailed => Self::L_REG_FAILED.to_smolstr(),
            LocalResponse::UnexpectedSendResp => Self::UNEXPECTED_SEND_RESP.to_smolstr(),
            LocalResponse::ReplacedAddress(addr) => {
                smol_str::format_smolstr!("{} {}", Self::REPLACED, addr)
            }
            LocalResponse::UnregisteredHostname => Self::UNREGISTERED_HOSTNAME.to_smolstr(),
            LocalResponse::AnyPathInvalid => Self::ANY_PATH_INVALID.to_smolstr(),
            LocalResponse::UnexpectedRemoteResponse => Self::UNEXPECTED_REMOTE_RESP.to_smolstr(),
        }
    }
}

impl LocalResponse {
    pub fn to_str_unchecked(self) -> &'static str {
        match self {
            LocalResponse::RemoteUnregistered => Self::R_UNREG_HOST,
            LocalResponse::RemoteNoAvailablePort => Self::R_NO_AVAILABLE_PORT,
            LocalResponse::AllFilesSucceeded => Self::ALL_FILES_SUCCEEDED,
            LocalResponse::LocalRegisterFailed => Self::L_REG_FAILED,
            LocalResponse::UnexpectedSendResp => Self::UNEXPECTED_SEND_RESP,
            LocalResponse::UnregisteredHostname => Self::UNREGISTERED_HOSTNAME,
            LocalResponse::AnyPathInvalid => Self::ANY_PATH_INVALID,
            LocalResponse::UnexpectedRemoteResponse => Self::UNEXPECTED_REMOTE_RESP,
            _ => "",
        }
    }
}

impl LocalResponse {
    const FILE_INFO: &'static str = "FILE_INFO";
    const R_UNREG_HOST: &'static str = "R_UNREG_HOST";
    const L_REG_FAILED: &'static str = "L_REG_FAILED";
    const R_NO_AVAILABLE_PORT: &'static str = "R_NO_AVAILABLE_PORT";

    const UNEXPECTED_REMOTE_RESP: &'static str = "UNEXPECTED_R_RESP";
    const UNREACHABLE: &'static str = "UNREACHABLE";
    const ALL_FILES_SUCCEEDED: &'static str = "ALL_FILES_SUCCEEDED";
    const PROGRESS: &'static str = "PROGRESS";
    const UNEXPECTED_SEND_RESP: &'static str = "UNEXPECTED_SEND_RESP";
    const FILES_SUCCEEDED: &'static str = "FILES_SUCCEEDED";
    const REPLACED: &'static str = "REPLACED";
    const UNREGISTERED_HOSTNAME: &'static str = "UNREG_HOSTNAME";
    const ANY_PATH_INVALID: &'static str = "ANY_PATH_INVALID";
}

#[derive(Debug, Clone)]
pub struct StartLine {
    tag: RequestCommand,
    extra_args: Option<SmolStr>,
}

impl StartLine {
    pub const LENGTH_LIMIT: u64 = 50;
}

#[derive(Debug, Clone)]
pub struct Request {
    start_line: StartLine,
    extra_data: Option<SmolStr>,
}

#[cfg(test)]
mod number_tests {}
