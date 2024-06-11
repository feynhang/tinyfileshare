use std::{net::IpAddr, path::PathBuf};

use crate::{consts, CommonResult};

#[derive(Debug)]
pub enum Response {
    Success(SuccessResponse),
    Failure(FailureResponse),
}

#[derive(Debug)]
pub enum SuccessResponse {
    HostReached,
    ListenerStarted,
    AllPathsReceived,
    InvalidPaths(Vec<String>),
    PortConfirm(u16),
    FileSendProgress(f64),
    ReceiveFinished,
    ReplacedHost(IpAddr),
    RegisterSuccess,
}

#[derive(Debug)]
pub enum FailureResponse {
    FileRecvFailed(FileRecvError),
    InvalidRequest,
    UnregisteredHost,
    AllPathsInvalid,
    ConnectionsOverload,
    ConnectHostFailed(String, IpAddr),
}

#[derive(Debug)]
pub enum FileRecvError {
    IncorrectFileSize,
}


impl std::fmt::Display for SuccessResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SuccessResponse::ListenerStarted => {
                write!(f, "{}", consts::reply::LISTENER_STARTED)
            }
            SuccessResponse::PortConfirm(port) => {
                write!(f, "{} {}", consts::reply::PORT_CONFIRM, *port)
            }
            SuccessResponse::FileSendProgress(progress) => write!(f, " {}", progress),
            SuccessResponse::RegisterSuccess => write!(f, ""),
            SuccessResponse::ReceiveFinished => {
                write!(f, "{}", consts::reply::RECV_FINISHED)
            }
            SuccessResponse::HostReached => write!(f, "{}", consts::reply::HOST_REACHED),
            SuccessResponse::InvalidPaths(paths) => write!(
                f,
                "{}\r\n\r\n{}",
                consts::reply::INVALID_PATHS,
                paths.as_slice().join("\r\n")
            ),
            SuccessResponse::ReplacedHost(ip) => {
                write!(f, "{} {}", consts::reply::REPLACED_IP, *ip)
            }
            SuccessResponse::AllPathsReceived => write!(f, "{}", consts::reply::ALL_PATHS_RECEIVED),
        }
    }
}

impl std::fmt::Display for FailureResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FailureResponse::InvalidRequest => {
                write!(f, "{}", consts::reply::INVALID_REQUEST)
            }
            FailureResponse::UnregisteredHost => {
                write!(f, "{}", consts::reply::UNREGISTERED_HOST)
            }
            FailureResponse::AllPathsInvalid => {
                write!(f, "{}", consts::reply::ALL_PATHS_INVALID)
            }
            FailureResponse::ConnectionsOverload => {
                write!(f, "{}", consts::reply::CONNECTIONS_OVERLOAD)
            }
            FailureResponse::ConnectHostFailed(hostname, ip) => {
                write!(
                    f,
                    "{} {}:{}",
                    consts::reply::CONNECT_HOST_FAILED,
                    hostname,
                    *ip
                )
            }
            FailureResponse::FileRecvFailed(_) => todo!(),
        }
    }
}

impl std::fmt::Display for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Response::Success(resp) => resp.fmt(f),
            Response::Failure(fail_resp) => fail_resp.fmt(f),
        }
    }
}
