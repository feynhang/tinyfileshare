use std::net::IpAddr;

use crate::response::Response;



#[derive(Debug)]
pub enum CommonError {
    IoErr(std::io::Error),
    PathErr(String),
    // InvalidRequest(&'static str),
    ReplyErr(Response),
    // SerializeError(toml::ser::Error),
    // DeserializeError(toml::de::Error),
    SimpleError(String),
}




impl std::fmt::Display for CommonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommonError::IoErr(io_err) => io_err.fmt(f),
            CommonError::SimpleError(e) => e.fmt(f),
            CommonError::PathErr(e) => e.fmt(f),
            CommonError::ReplyErr(reply_err) => std::fmt::Debug::fmt(reply_err, f),
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

