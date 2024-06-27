pub mod config;
pub mod request_tag;
pub mod response_tag;
pub mod server;

pub(crate) mod handler;

pub mod consts {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    pub const HOST_NAME_LENGTH_LIMIT: usize = 20;
    pub const PIAR_SEP: char = ' ';
    pub const PAIR_SEP: char = ':';
    pub(crate) const GET_HOME_DIR_FAILED: &str =
        "Unexpected: get home dir failed! Maybe you are in an unsupported platform!";

    pub const KB: usize = 1024;
    pub const MB: usize = usize::pow(KB, 2);
    pub const GB: usize = usize::pow(MB, 2);
    pub const DEFAULT_CONFIG_DIR_NAME: &str = ".tinyfileshare";
    pub const DEFAULT_CONFIG_FILE_NAME: &str = "config.toml";

    pub const FILE_NAME_LENGTH_LIMIT: usize = 260;

    pub const DEFAULT_IPC_SOCK_NAME: &str = "share.sock";
    pub const UNSPECIFIED_PORT: u16 = 0;
    pub const PORT_TEST_BOUND: u16 = 1000;

    pub const DEFAULT_LISTENER_ADDR: SocketAddr =
        SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 10020);
    pub const FILE_TRANS_BUF_SIZE: usize = 8192;

    pub const LINE_SEP: &str = "\r\n";
    pub const ASCII_SPACE: char = ' ';

    pub const PATHS_NUM_PER_REQUEST: usize = 4;

    pub const FILE_PATH_LIMIT: u64 = 500;

    pub const MAX_IP_LEN: u64 = 46;
}

mod global {
    use std::sync::OnceLock;

    use tokio::sync::RwLock;

    use crate::config::ConfigStore;
<<<<<<< HEAD
=======
    use crate::consts;
>>>>>>> c22d847 (	modified:   Cargo.lock)

    pub(crate) async fn config_store() -> &'static RwLock<ConfigStore> {
        static mut CONFIG: OnceLock<RwLock<ConfigStore>> = OnceLock::new();
        unsafe {
            match CONFIG.get_mut() {
                Some(conf_store_lock) => {
                    let mut config_store = conf_store_lock.write().await;
                    if let Err(e) = config_store.try_update_from_file() {
                        log::error!(
                            "Update config in config_store failed and ignored it! Detail: {}",
                            e
                        );
                    }
                    conf_store_lock
                }
<<<<<<< HEAD
                None => CONFIG.get_or_init(|| RwLock::new(ConfigStore::default())),
            }
        }
    }
=======
                None => {
                    CONFIG.get_or_init(|| RwLock::new(ConfigStore::default()))
                }
            }
        }
    }

    static mut IPC_SVR_SOCK_NAME: SmolStr =
        SmolStr::new_inline(consts::DEFAULT_SERVER_IPC_SOCK_NAME);

    static mut IPC_CLT_SOCK_NAME: SmolStr =
        SmolStr::new_inline(consts::DEFAULT_CLIENT_IPC_SOCK_NAME);
    pub(crate) fn server_ipc_sock_name() -> &'static str {
        unsafe { IPC_SVR_SOCK_NAME.as_str() }
    }

    pub(crate) fn client_ipc_sock_name() -> &'static str {
        unsafe { IPC_CLT_SOCK_NAME.as_str() }
    }

    pub(crate) fn set_server_ipc_sock_name(server_socket_name: SmolStr) {
        unsafe {
            IPC_SVR_SOCK_NAME = server_socket_name;
        }
    }

    pub(crate) fn set_client_ipc_sock_name(client_socket_name: SmolStr) {
        unsafe {
            IPC_CLT_SOCK_NAME = client_socket_name;
        }
    }
>>>>>>> c22d847 (	modified:   Cargo.lock)
}

#[cfg(test)]
mod tests {}
