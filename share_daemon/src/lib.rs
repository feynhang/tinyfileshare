pub mod config;
pub mod server;

pub(crate) mod handler;

pub mod consts {
    use std::{
        net::{IpAddr, Ipv4Addr, SocketAddr},
        time::Duration,
    };
    pub(crate) const GET_HOME_DIR_FAILED: &str =
        "Unexpected: get home dir failed! Maybe you are in an unsupported platform!";

    pub const KB: usize = 1024;
    pub const MB: usize = usize::pow(KB, 2);
    pub const GB: usize = usize::pow(MB, 2);
    pub const DEFAULT_CONFIG_DIR_NAME: &str = ".tinyfileshare";
    pub const DEFAULT_CONFIG_FILE_NAME: &str = "config.toml";

    pub const FILE_NAME_LENGTH_LIMIT: usize = 260;
    // pub

    pub const DEFAULT_CLIENT_IPC_SOCKET_NAME: &str = "share-client.sock";
    pub const DEFAULT_SERVER_IPC_SOCKET_NAME: &str = "share-server.sock";
    pub const UNSPECIFIED_PORT: u16 = 0;
    pub const PORT_TEST_BOUND: u16 = 1000;

    pub const DEFAULT_LISTENER_ADDR: SocketAddr =
        SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), UNSPECIFIED_PORT);
    pub const FILE_TRANS_BUF_SIZE: usize = 8192;

    pub const LINE_SEP: &str = "\r\n";
    pub const NEWLINE: u8 = b'\n';
    pub const ASCII_SPACE: u8 = b' ';

    pub const PATHS_NUM_PER_REQUEST: usize = crate::config::DEFAULT_NUM_WORKERS as usize - 1;

    pub const FILE_PATH_LIMIT: u64 = 500;
    pub const MAX_HOSTNAME_LIMIT: u64 = 16;
    pub const MAX_IP_LEN: u64 = 46;

    pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

    pub mod trans_flag {
        pub const TRANSFER_START: &str = "TRANSFER_START";
        pub const TRANSFER_END: &str = "TRANSFER_END";
    }

    pub mod request {
        pub const SHARE: &str = "SHARE";
        pub const HOST_REG: &str = "REG";
        pub const PORT_EXPECTED: &str = "PORT";
        pub const REG_ME: &str = "REG_ME";
        pub const REG_FROM: &str = "REG_FROM";
        pub const FILES_RECV: &str = "FILES_RECV";
    }
    pub mod reply {
        pub const ACCEPT: &str = "ACCEPT";
        pub const REJECT: &str = "REJECT";
        pub const INVALID_RECV_DIR: &str = "INVALID_RECV_DIR";

        /// which means the remote host can not accept a request
        pub const REMOTE_STREAM_UNWRITABLE: &str = "REMOTE_STREAM_UNWRITABLE";
        pub const REMOTE_UNRESPONSIVE: &str = "REMOTE_UNRESPONSIVE";
        pub const UNEXPECTED_REMOTE_RESPONSE: &str = "UNEXPECTED_REMOTE_RESPONSE";
        pub const UNEXPECTED_RESPONSE: &str = "UNEXPECTED_RESPONSE";
        pub const UNREACHABLE_ADDRESS: &str = "UNREACHABLE_ADDRESS";
        pub const PROGRESS: &str = "PROGRESS";
        pub const REPLACED_IP: &str = "REPLACED";
        pub const RECV_FINISHED: &str = "RECV_FINISHED";
        pub const INVALID_REQUEST: &str = "INVALID_REQUEST";
        pub const CONNECTIONS_OVERLOAD: &str = "CONNECTION_OVERLOAD";
        pub const LISTENER_STARTED: &str = "LISTENER_STARTED";
        pub const PORT_CONFIRM: &str = "PORT_CONFIRM";
        pub const ALL_PATHS_INVALID: &str = "ALL_PATHS_INVALID";
        pub const ANY_PATH_INVALID: &str = "INVALID_PATHS";
        pub const ALL_PATHS_RECEIVED: &str = "ALL_PATHS_RECEIVED";
        pub const CONNECT_HOST_FAILED: &str = "CONNECT_HOST_FAILED";
        pub const UNREGISTERED_HOST: &str = "UNREGISTERED_HOST";
        pub const UNREGISTERED_LOCAL: &str = "UNREGISTERED_LOCAL";
        pub const UNREGISTERED_REMOTE: &str = "UNREGISTERED_REMOTE";
        pub const REMOTE_REGISTRATION_UNSUPPORTED: &str = "REMOTE_REG_UNSUPPORTED";
        pub const REMOTE_REGISTRATION_REFUSED: &str = "REMOTE_REG_REFUSED";
        pub const TRANS_REMOTE_REFUSED: &str = "TRANS_REMOTE_REFUSED";
        pub const INVALID_PORT: &str = "INVALID_PORT";
        pub const REMOTE_NO_PORT_AVAILABLE: &str = "REMOTE_NO_PORT_AVAILABLE";
        pub const NO_PORT_AVAILABLE: &str = "NO_PORT_AVAILABLE";
        // pub const WAITING: &str = "WAITING";
        pub const REMOTE_REGISTRATION_FAILED: &str = "REMOTE_REG_FAILED";
        pub const CLIENT_REGISTRATION_FAILED: &str = "CLIENT_REG_FAILED";
        pub const REGISTRATION_SUCCEEDED: &str = "REG_SUCCEEDED";
        pub const LOCAL_REGISTRATION_FAILED: &str = "LOCAL_REG_FAILED";
        pub const REGISTRATION_REFUSED: &str = "REG_REFUSED";
        pub const ALL_FILES_SUCCEEDED: &str = "ALL_FILES_SUCCEEDED";
        pub const FILES_SUCCEEDED: &str = "FILES_SUCCEEDED";
        pub const UNEXPECTED_END_FLAG: &str = "UNEXPECTED_END_FLAG";
        pub const UNEXPECTED_SEND_RESPONSE: &str = "UNEXPECTED_SEND_RESP";
        pub const RECV_REFUSED: &str = "RECV_REFUSED";
        pub const RECEIVED: &str = "RECEIVED";
    }
}

mod global {
    use std::ffi::OsStr;
    use std::fs::File;
    use std::path::{Path, PathBuf};
    use std::sync::OnceLock;

    use smol_str::{format_smolstr, SmolStr};
    use tokio::sync::RwLock;

    use crate::config::ConfigStore;
    use crate::consts;

    // pub const BUF_SIZE: usize = 4096;
    // pub const NEWLINE: &str = "\r\n";

    static mut CONFIG_PATH: Option<PathBuf> = None;
    static mut LOG_DIR: Option<PathBuf> = None;

    pub(crate) fn log_dir() -> &'static Path {
        unsafe { LOG_DIR.get_or_insert(default_log_dir()) }
    }

    pub(crate) fn set_log_dir(dir_path: PathBuf) {
        unsafe {
            LOG_DIR = Some(checked_log_dir(dir_path));
        }
    }

    fn checked_log_dir(path: PathBuf) -> PathBuf {
        if path.is_dir() {
            return path;
        }
        if path.is_file() || path.is_symlink() || path.extension().is_some() {
            return default_log_dir();
        }
        if !path.exists() {
            std::fs::create_dir_all(&path).expect("Create log dir failed!!!");
        }
        path
    }
    
    pub(crate) fn open_log_file() -> File {
        let mut log_file_path = log_dir().to_owned();
        log_file_path.push("tinyfileshare.log");
        let mut open_options = File::options();
        if log_file_path.exists() {
            open_options.append(true);
        } else {
            open_options.write(true).create(true);
        }
        match open_options.open(log_file_path) {
            Ok(f) => f,
            Err(e) => panic!(
                "Error occurred while open or create log file! Detail: {}",
                e
            ),
        }
    }


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
        SmolStr::new_inline(consts::DEFAULT_SERVER_IPC_SOCKET_NAME);
    static mut IPC_CLT_SOCK_NAME: SmolStr =
        SmolStr::new_inline(consts::DEFAULT_CLIENT_IPC_SOCKET_NAME);
    pub(crate) fn server_ipc_socket_name() -> &'static str {
        unsafe { IPC_SVR_SOCK_NAME.as_str() }
    }

    pub(crate) fn client_ipc_socket_name() -> &'static str {
        unsafe { IPC_CLT_SOCK_NAME.as_str() }
    }

    pub(crate) fn set_server_ipc_socket_name(name: SmolStr) {
        unsafe {
            IPC_SVR_SOCK_NAME = name;
        }
    }

    pub(crate) fn set_client_ipc_socket_name(name: SmolStr) {
        unsafe {
            IPC_CLT_SOCK_NAME = name;
        }
    }

    pub fn exec_dir_path() -> &'static Path {
        static EXE_DIR: OnceLock<PathBuf> = OnceLock::new();
        EXE_DIR.get_or_init(|| {
            let exe_path = std::env::current_exe().unwrap();
            let str_exe_path = exe_path.to_str().unwrap();
            PathBuf::from(&str_exe_path[0..str_exe_path.rfind(std::path::MAIN_SEPARATOR).unwrap()])
        })
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
                return Err(anyhow::Error::msg(format_smolstr!(
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

    pub(crate) fn default_log_dir() -> PathBuf {
        let mut default_log_dir = exec_dir_path().to_path_buf();
        default_log_dir.push("log");
        std::fs::create_dir_all(&default_log_dir)
            .expect("Unexpected error occurred while create log dir!");
        default_log_dir
    }
}

#[cfg(test)]
mod tests {}
