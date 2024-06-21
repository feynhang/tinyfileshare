pub mod config;
pub mod request_tag;
pub mod response_tag;
pub mod server;

pub(crate) mod handler;

pub mod consts {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    pub const STARTLINE_PARTS_SEP: char = ' ';
    pub const PAIR_SEP: char = ':';
    pub const DEFAULT_LISTENER_PORT: u16 = 10020;
    pub(crate) const GET_HOME_DIR_FAILED: &str =
        "Unexpected: get home dir failed! Maybe you are in an unsupported platform!";

    pub const KB: usize = 1024;
    pub const MB: usize = usize::pow(KB, 2);
    pub const GB: usize = usize::pow(MB, 2);
    pub const DEFAULT_CONFIG_DIR_NAME: &str = ".tinyfileshare";
    pub const DEFAULT_CONFIG_FILE_NAME: &str = "config.toml";

    pub const FILE_NAME_LENGTH_LIMIT: usize = 260;

    pub const DEFAULT_CLIENT_IPC_SOCK_NAME: &str = "share-client.sock";
    pub const DEFAULT_SERVER_IPC_SOCK_NAME: &str = "share-server.sock";
    pub const UNSPECIFIED_PORT: u16 = 0;
    pub const PORT_TEST_BOUND: u16 = 1000;

    pub const DEFAULT_LISTENER_ADDR: SocketAddr =
        SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), DEFAULT_LISTENER_PORT);
    pub const FILE_TRANS_BUF_SIZE: usize = 8192;

    pub const LINE_SEP: &str = "\r\n";
    pub const NEWLINE: u8 = b'\n';
    pub const ASCII_SPACE: u8 = b' ';

    pub const PATHS_NUM_PER_REQUEST: usize = crate::config::DEFAULT_NUM_WORKERS as usize - 1;

    pub const FILE_PATH_LIMIT: u64 = 500;
    pub const HOSTNAME_LEN_LIMIT: u64 = 16;
    pub const MAX_IP_LEN: u64 = 46;
}

mod global {
    use std::ffi::OsStr;
    use std::path::{Path, PathBuf};
    use std::sync::OnceLock;

    use smol_str::SmolStr;
    use tokio::sync::RwLock;

    use crate::config::ConfigStore;
    use crate::consts;

    static mut CONFIG_PATH: Option<PathBuf> = None;

    pub(crate) async fn config_store() -> &'static RwLock<ConfigStore> {
        static mut CONFIG: OnceLock<RwLock<ConfigStore>> = OnceLock::new();
        unsafe {
            match CONFIG.get_mut() {
                Some(conf_store_lock) => {
                    let mut config_store = conf_store_lock.write().await;
                    if let Err(e) = config_store.try_update_config() {
                        log::error!("Update config in config_store failed! Detail: {}", e);
                    }
                    conf_store_lock
                }
                None => {
                    let c = ConfigStore::from_config_file();
                    let c_lock = RwLock::new(c);
                    CONFIG.get_or_init(|| c_lock)
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

    pub(crate) fn set_server_ipc_sock_name(name: SmolStr) {
        unsafe {
            IPC_SVR_SOCK_NAME = name;
        }
    }

    pub(crate) fn set_client_ipc_sock_name(name: SmolStr) {
        unsafe {
            IPC_CLT_SOCK_NAME = name;
        }
    }

    pub(crate) fn set_config_path(path: PathBuf) -> anyhow::Result<()> {
        unsafe {
            CONFIG_PATH = Some(check_path(path)?);
        }
        Ok(())
    }

    fn check_path(mut path_for_check: PathBuf) -> anyhow::Result<PathBuf> {
        if path_for_check.is_symlink() {
            return Err(anyhow::Error::msg(
                "Symbolic link for config path is not supported!",
            ));
        }

        if path_for_check.is_file() || path_for_check == default_config_path() {
            return Ok(path_for_check);
        }

        fn try_create_dir(dir_path: &Path) -> anyhow::Result<()> {
            if !dir_path.exists() && std::fs::create_dir_all(dir_path).is_err() {
                return Err(anyhow::Error::msg(smol_str::format_smolstr!(
                    "Create dir failed: {}, please check it validity!",
                    dir_path.to_string_lossy()
                )));
            }
            Ok(())
        }

        if let Some(ext_name) = path_for_check.extension() {
            if ext_name == OsStr::new("toml") {
                path_for_check.pop();
                try_create_dir(&path_for_check)?;
                return Ok(path_for_check);
            }
        }
        if !path_for_check.is_dir() {
            try_create_dir(&path_for_check)?;
        }
        path_for_check.push(consts::DEFAULT_CONFIG_FILE_NAME);
        Ok(path_for_check)
    }

    pub(crate) fn config_path() -> &'static Path {
        unsafe { CONFIG_PATH.get_or_insert(default_config_path().to_path_buf()) }
    }

    pub(crate) fn default_config_path() -> &'static Path {
        static mut DEFAULT_CONFIG_PATH: OnceLock<PathBuf> = OnceLock::new();
        unsafe {
            DEFAULT_CONFIG_PATH.get_or_init(|| {
                let mut path = dirs::home_dir().expect(consts::GET_HOME_DIR_FAILED);
                path.push(consts::DEFAULT_CONFIG_DIR_NAME);
                if !path.exists() {
                    std::fs::create_dir_all(&path).unwrap();
                }
                path.push(consts::DEFAULT_CONFIG_FILE_NAME);
                path
            })
        }
    }
}

#[cfg(test)]
mod tests {}
