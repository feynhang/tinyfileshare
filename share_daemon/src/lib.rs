pub mod common;
pub mod config;
pub mod request_tag;
pub mod server;

pub(crate) mod handler;

pub mod consts {
    use std::{
        net::{IpAddr, Ipv4Addr, SocketAddr},
        time::Duration,
    };
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    pub const FILE_SIZE_LIMIT: u64 = 10 * GB;
    pub const DEFAULT_CONFIG_DIR_NAME: &str = ".tinyfileshare";
    pub const DEFAULT_CONFIG_FILE_NAME: &str = "config.toml";
    pub const MIN_PORT: u16 = 3000;
    const DEFAULT_PORT: u16 = 10020;
    pub const HOST_CHECK_TIMEOUT: Duration = Duration::from_secs(15);
    pub const DEFAULT_LISTENER_ADDR: SocketAddr =
        SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), DEFAULT_PORT);
    pub const UNSPECIFIED_LISTENER_ADDR: SocketAddr =
        SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);
    pub const DEFAULT_IPC_SOCK_NAME: &str = "share.sock";
    pub const HOST_NAME_LENGTH_LIMIT: usize = 20;
    pub const LINE_SEP: &str = "\r\n";
    pub const ASCII_SPACE: char = ' ';
    pub const STARTLINE_SEP: char = ' ';
    pub const PAIR_SEP: char = ':';
    pub const FILE_NAME_LENGTH_LIMIT: usize = 260;
    pub const NUMBER_PATHS_PER_REQUEST: usize = 4;
    pub const FILE_TRANS_BUF_SIZE: usize = 8192;
    pub const FILE_PATH_LIMIT: u64 = 500;
    pub const DEFAULT_NUM_WORKERS: u8 = 4;
}

mod global {
    use std::sync::OnceLock;

    use tokio::sync::RwLock;

    use crate::config::ConfigStore;

    pub(crate) async fn config_store() -> &'static RwLock<ConfigStore> {
        static mut CONFIG: OnceLock<RwLock<ConfigStore>> = OnceLock::new();
        unsafe {
            match CONFIG.get_mut() {
                Some(conf_store_lock) => {
                    let mut config_store = conf_store_lock.write().await;
                    if let Err(e) = config_store.try_update_from_file() {
                        log::error!("Try to update from config file failed! Detail: {}", e);
                    }
                    conf_store_lock
                }
                None => CONFIG.get_or_init(|| RwLock::new(ConfigStore::default())),
            }
        }
    }
}

#[cfg(test)]
mod tests {}
