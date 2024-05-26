use error::ServerError;

pub mod config;
pub(crate) mod dispatcher;
pub mod error;
pub(crate) mod filedata;
pub(crate) mod handler;
#[allow(unused)]
pub(crate) mod log;
pub mod server;
pub(crate) mod util;
pub(crate) mod workers;

pub type ServerResult<T> = Result<T, ServerError>;

// pub type CommonResult<T> = Result<T, Box<dyn std::error::Error>>;

mod global {
    use std::path::{Path, PathBuf};

    pub const DEFAULT_CONFIG_DIR_NAME: &str = ".tinyfileshare";
    pub const DEFAULT_CONFIG_FILE_NAME: &str = "config.toml";
    static mut CONFIG_PATH: Option<PathBuf> = None;

    pub(crate) fn set_config_path<T: Into<PathBuf>>(path: T) {
        unsafe {
            CONFIG_PATH = Some(path.into());
        }
    }
    pub(crate) fn config_path() -> &'static Path {
        unsafe { CONFIG_PATH.get_or_insert(default_config_path()) }
    }

    pub(crate) fn default_config_path() -> PathBuf {
        let mut path = crate::handler::PathHandler::get_home_path();
        path.push(DEFAULT_CONFIG_DIR_NAME);
        path
    }
}

#[cfg(test)]
mod tests {}
