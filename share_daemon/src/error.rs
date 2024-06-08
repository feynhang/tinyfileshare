use crate::response::FailureResponse;

#[derive(Debug)]
pub enum CommonError {
    IpcErr(IpcError),
    IoErr(std::io::Error),
    ConfigPathErr(String),
    FailureResp(FailureResponse),
    SimpleError(String),
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

impl From<FailureResponse> for CommonError {
    fn from(value: FailureResponse) -> Self {
        Self::FailureResp(value)
    }
}

fn to_lines_string<T: AsRef<str>>(vec: &Vec<T>) -> String {
    let mut ret = String::new();
    for d in vec {
        ret.push_str(d.as_ref());
    }
    ret
}

impl std::fmt::Display for CommonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommonError::IoErr(io_err) => io_err.fmt(f),
            CommonError::SimpleError(e) => e.fmt(f),
            // CommonError::InvalidPaths(e) => write!(f, "SOME_PATHS_INVALID:\n{}", to_lines_string(e)),
            CommonError::FailureResp(reply_err) => std::fmt::Debug::fmt(reply_err, f),
            CommonError::ConfigPathErr(extra_msg) => {
                if extra_msg.trim().is_empty() {
                    write!(f, "Path Error")
                } else {
                    write!(f, "Path Error: {}", extra_msg)
                }
            }
            CommonError::IpcErr(ipc_err) => ipc_err.fmt(f),
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
