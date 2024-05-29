#[derive(Debug)]
pub enum ResponseCode {
    InvalidRequest,
    InvalidTransFormat,
    PathsAccepted,
    RegisterSuccess,
    FileTransProgress(f64),
    FileTransEmpty,
    RegisterFailed,
    UnRegistered,
    ConnectionsExceedsLimit,
    FileTransFailed(std::io::Error),
}

impl std::fmt::Display for ResponseCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResponseCode::RegisterFailed => write!(f, "REG_FAILED"),
            ResponseCode::UnRegistered => write!(f, "UNREGISTERED"),
            ResponseCode::ConnectionsExceedsLimit => write!(f, "CONNS_EXCEEDS_LIMIT"),
            ResponseCode::PathsAccepted => write!(f, "PATHS_ACCEPTED"),
            ResponseCode::RegisterSuccess => write!(f, "REGISTER_SUCCESS"),
            ResponseCode::FileTransProgress(progress) => write!(f, "PROGRESS:{}", progress),
            ResponseCode::InvalidRequest => write!(f, "INVALID_REQUEST"),
            ResponseCode::FileTransFailed(inner_err) => write!(f, "FILE_TRANS_FAILED\nDETAIL: {}", inner_err),
            ResponseCode::InvalidTransFormat => write!(f, "INVALID_TRANS_FORMAT"),
            ResponseCode::FileTransEmpty => write!(f, "FILE_TRANS_EMPTY"),
        }
    }
}
