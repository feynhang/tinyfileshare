use std::str::Utf8Error;

use crate::consts;


#[derive(Debug)]
pub enum CommonError {
    IpcErr(IpcError),
    FailureResponse(&'static str),
    IoErr(std::io::Error),
    ConfigPathErr(String),
    Utf8Err(Utf8Error),
    DeserializeErr(toml::de::Error),
    SimpleError(String),
    Failed,
}

#[derive(Debug)]
pub enum IpcError {
    AddrInUse(&'static str),
}

impl std::fmt::Display for IpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IpcError::AddrInUse(name) => write!(
                f,
                "Could not start server because the socket file is occupied. Please check if {} is in use by another process and try again.",
                name
            ),
        }
    }
}

impl From<toml::de::Error> for CommonError {
    fn from(value: toml::de::Error) -> Self {
        Self::DeserializeErr(value)
    }
}

impl From<Utf8Error> for CommonError {
    fn from(value: Utf8Error) -> Self {
        Self::Utf8Err(value)
    }
}


// fn to_lines_string<T: AsRef<str>>(vec: &Vec<T>) -> String {
//     let mut ret = String::new();
//     for d in vec {
//         ret.push_str(d.as_ref());
//     }
//     ret
// }

impl std::fmt::Display for CommonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommonError::IoErr(io_err) => io_err.fmt(f),
            CommonError::SimpleError(e) => e.fmt(f),
            CommonError::ConfigPathErr(extra_msg) => {
                if extra_msg.trim().is_empty() {
                    write!(f, "Path Error")
                } else {
                    write!(f, "Path Error: {}", extra_msg)
                }
            }
            CommonError::IpcErr(ipc_err) => ipc_err.fmt(f),
            CommonError::Utf8Err(utf8_err) => utf8_err.fmt(f),
            CommonError::DeserializeErr(deser_err) => deser_err.fmt(f),
            CommonError::FailureResponse(resp_str) => write!(f, "{}{}", resp_str, consts::LINE_SEP),
            CommonError::Failed => write!(f, "ActionFailed"),
            // CommonError::RequestErr(e) => std::fmt::Debug::fmt(e, f),
            // CommonError::InvalidRequest(detail) => write!(f,"INVALID_REQUEST:{}", detail),
            // CommonError::ConnectionsExceedsLimit => write!(f, "CONNECTION_EXCEEDS_LIMIT"),
            // CommonError::RegisterFailed => write!(f, "REG_FAILED"),
            // CommonError::UnRegistered => write!(f, "UNREGISTERED"),
            // CommonError::RegisterFailed(detail) => write!(f, "{}", detail),
            // CommonError::DeserializeError(deser_err) => deser_err.fmt(f),
        }
    }
}

impl std::error::Error for CommonError {}

// impl From<&'static str> for CommonError {
//     fn from(value: &'static str) -> Self {
//         Self::SimpleError(value.to_owned())
//     }
// }

// impl From<String> for CommonError {
//     fn from(value: String) -> Self {
//         Self::SimpleError(value)
//     }
// }

impl From<std::io::Error> for CommonError {
    fn from(value: std::io::Error) -> Self {
        Self::IoErr(value)
    }
}
