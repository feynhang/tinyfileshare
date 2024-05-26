#[derive(Debug)]
pub enum ServerError {
    InvalidRequest,
    IoError(std::io::Error),
    PathError(&'static str),
    ConfigError(&'static str),
    // SerializeError(toml::ser::Error),
    DeserializeError(toml::de::Error),
    CommonError(String),
}

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerError::InvalidRequest => write!(f, "Invalid request!!!"),
            ServerError::IoError(io_err) => io_err.fmt(f),
            ServerError::CommonError(e) => e.fmt(f),
            ServerError::ConfigError(msg) => msg.fmt(f),
            ServerError::PathError(e) => e.fmt(f),
            ServerError::DeserializeError(deser_err) => deser_err.fmt(f),
        }
    }
}

impl std::error::Error for ServerError {}

impl From<&'static str> for ServerError {
    fn from(value: &'static str) -> Self {
        Self::CommonError(value.to_owned())
    }
}

impl From<String> for ServerError {
    fn from(value: String) -> Self {
        Self::CommonError(value)
    }
}

impl From<std::io::Error> for ServerError {
    fn from(value: std::io::Error) -> Self {
        Self::IoError(value)
    }
}

impl From<toml::de::Error> for ServerError {
    fn from(value: toml::de::Error) -> Self {
        Self::DeserializeError(value)
    }
}
