pub(crate) mod util;
pub mod config;
pub(crate) mod dispatcher;
#[allow(unused)]
pub(crate) mod log;
pub(crate) mod workers;
pub(crate) mod filedata;
pub(crate) mod handler;
pub mod server;
pub(crate) mod session;
pub mod svr_err;

use handler::PathHandler;
use server::Server;
use std::{
    net::{IpAddr, Ipv4Addr},
    path::{Path, PathBuf},
    sync::OnceLock,
};

pub type CommonResult<T> = Result<T, Box<dyn std::error::Error>>;

static mut CONFIG_PATH: Option<PathBuf> = None;
pub(crate) fn set_config_path(path: &Path) {
    unsafe {
        CONFIG_PATH = Some(path.to_owned());
    }
}

pub(crate) fn config_path() -> &'static Path {
    unsafe {
        match CONFIG_PATH {
            Some(ref path) => path,
            None => {
                let mut path = PathHandler::get_home_path();
                path.push(".tinyfileshare");
                path.push("config.ini");
                CONFIG_PATH = Some(path);
                CONFIG_PATH.as_ref().unwrap()
            }
        }
    }
}

pub const LOCALHOST: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
#[cfg(test)]
mod tests {
    // use crate::get_home_path;

    use crate::handler::PathHandler;

    #[test]
    fn test_get_home() {
        assert_eq!(
            std::path::Path::new("C:\\Users\\feyn"),
            PathHandler::get_home_path()
        )
    }
}
