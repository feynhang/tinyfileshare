use error::CommonError;

pub mod config;

pub mod error;
pub(crate) mod filedata;
pub(crate) mod handler;
pub(crate) mod log;
pub mod server;
pub(crate) mod util;

pub type CommonResult<T> = Result<T, CommonError>;

// pub type CommonResult<T> = Result<T, Box<dyn std::error::Error>>;

pub mod consts {
    use std::time::Duration;

    pub const KB: usize = 1024;
    pub const MB: usize = usize::pow(KB, 2);
    pub const GB: usize = usize::pow(MB, 2);
    pub const MAX_FILE_SIZE_PER_TRANS: usize = 10 * MB;

    pub const NAME_IPC_CLIENT: &str = "share_client.sock";
    pub const NAME_IPC_LISTENER: &str = "share_server.sock";

    pub const FILE_TRANS_BUF_SIZE: usize = 8192;

    pub const LINE_SEP: &str = "\r\n";
    pub const NEWLINE: u8 = b'\n';
    pub const ASCII_SPACE: u8 = b' ';

    pub const PATHS_NUM_PER_REQUEST: usize = crate::config::DEFAULT_NUM_WORKERS as usize - 1;

    pub const FILE_PATH_LIMIT: u64 = 500;
    pub const MAX_HOSTNAME_LIMIT: u64 = 16;
    pub const MAX_IP_LEN: u64 = 46;

    pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

    pub mod request {
        pub const TRANSFER_START: &str = "TRANSFER_START";
        pub const SHARE: &str = "SHARE";
        pub const HOST_REG: &str = "REG";
        pub const PORT_PREPARE: &str = "PORT";
        pub const REG_ME: &str = "REG_ME";
    }
    pub mod reply {

        pub const TRANSFER_END: &str = "TRANSFER_END";
        pub const UNEXPECTED_REMOTE_RESPONSE: &str = "UNEXPECTED_REMOTE_RESPONSE";
        pub const UNEXPECTED_RESPONSE: &str = "UNEXPECTED_RESPONSE";
        pub const UNREACHABLE_SOCKETADDRESS: &str = "UNREACHABLE_IP";
        pub const PROGRESS: &str = "PROGRESS";
        pub const REPLACED_IP: &str = "REPLACED";
        pub const RECV_FINISHED: &str = "RECV_FINISHED";
        pub const INVALID_REQUEST: &str = "INVALID_REQUEST";
        pub const CONNECTIONS_OVERLOAD: &str = "CONNECTION_OVERLOAD";
        pub const LISTENER_STARTED: &str = "LISTENER_STARTED";
        pub const PORT_CONFIRM: &str = "PORT_CONFIRM";
        pub const PORT_OK: &str = "PORT_OK";
        pub const ALL_PATHS_INVALID: &str = "ALL_PATHS_INVALID";
        pub const THERES_INVALID_PATHS: &str = "INVALID_PATHS";
        pub const ALL_PATHS_RECEIVED: &str = "ALL_PATHS_RECEIVED";
        pub const CONNECT_HOST_FAILED: &str = "CONNECT_HOST_FAILED";
        pub const UNREGISTERED_REMOTE: &str = "UNREGISTERED_REMOTE";
        pub const UNREGISTERED_LOCAL: &str = "UNREGISTERED_LOCAL";
        pub const WAITING: &str = "WAITING";
        pub const REMOTE_REGISTRATION_FAILED: &str = "REMOTE_REGISTRATION_FAILED";
        pub const REGISTRATION_SUCCEED: &str = "REGISTRATION_SUCCEED";
        pub const LOCAL_REGISTRATION_FAILED: &str = "LOCAL_REGISTRATION_FAILED";
    }
}

mod global {
    use std::ffi::OsStr;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, OnceLock};

    use tokio::sync::RwLock;

    use crate::config::ConfigStore;
    use crate::error::CommonError;
    use crate::log::Logger;

    use crate::CommonResult;

    // pub const BUF_SIZE: usize = 4096;
    // pub const NEWLINE: &str = "\r\n";
    pub const DEFAULT_CONFIG_DIR_NAME: &str = ".tinyfileshare";
    pub const DEFAULT_CONFIG_FILE_NAME: &str = "config.toml";
    static mut CONFIG_PATH: Option<PathBuf> = None;

    static HOME_PATH: OnceLock<PathBuf> = OnceLock::new();
    pub(crate) static mut GLOBAL_LOGGER: Logger = Logger::console_logger();

    #[cfg(windows)]
    pub(crate) fn home_path() -> &'static Path {
        HOME_PATH.get_or_init(|| PathBuf::from(std::env::var("USERPROFILE").unwrap()))
    }

    #[cfg(not(windows))]
    pub(crate) fn home_path() -> PathBuf {
        HOME_PATH.get_or_init(PathBuf::from(std::env::var("HOME").unwrap()))
    }

    pub(crate) fn log_dir() -> &'static mut PathBuf {
        static mut LOG_DIR: Option<PathBuf> = None;
        unsafe { LOG_DIR.get_or_insert(default_log_dir().to_owned()) }
    }

    // pub(crate) fn fallback_to_default_config_store()

    pub(crate) async fn config_store() -> &'static RwLock<ConfigStore> {
        static mut CONFIG: Option<RwLock<ConfigStore>> = None;
        unsafe {
            match CONFIG.as_mut() {
                Some(conf_store_lock) => {
                    let mut config_store = conf_store_lock.write().await;
                    if let Err(e) = config_store.try_update_config().await {
                        logger()
                            .error(smol_str::format_smolstr!(
                                "Update config in config_store failed! Detail: {}",
                                e
                            ))
                            .await;
                    }
                    conf_store_lock
                }
                None => CONFIG
                    .get_or_insert(RwLock::new(ConfigStore::from_config_file().await)),
            }
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

    pub(crate) fn set_config_path(path: PathBuf) -> CommonResult<()> {
        unsafe {
            CONFIG_PATH = Some(check_path(path)?);
        }
        Ok(())
    }

    fn check_path(mut path_for_check: PathBuf) -> CommonResult<PathBuf> {
        if path_for_check.is_symlink() {
            return Err(CommonError::ConfigPathErr(format!(
                "Symbolic link for config path is not supported!"
            )));
        }
        if path_for_check.is_file() || path_for_check == default_config_path() {
            return Ok(path_for_check);
        }

        fn try_create_dir(dir_path: &Path) -> CommonResult<()> {
            if !dir_path.exists() && std::fs::create_dir_all(&dir_path).is_err() {
                return Err(CommonError::ConfigPathErr(format!(
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
        path_for_check.push(DEFAULT_CONFIG_FILE_NAME);
        Ok(path_for_check)
    }

    pub(crate) fn config_path() -> &'static Path {
        unsafe { CONFIG_PATH.get_or_insert(default_config_path().to_path_buf()) }
    }

    pub(crate) fn default_config_path() -> &'static Path {
        static mut DEFAULT_CONFIG_PATH: OnceLock<PathBuf> = OnceLock::new();
        unsafe {
            DEFAULT_CONFIG_PATH.get_or_init(|| {
                let mut path = home_path().to_owned();
                path.push(DEFAULT_CONFIG_DIR_NAME);
                if !path.exists() {
                    std::fs::create_dir_all(&path).unwrap();
                }
                path.push(DEFAULT_CONFIG_FILE_NAME);
                path
            })
        }
    }

    pub(crate) fn default_log_dir() -> &'static Path {
        static mut DEFAULT_LOG_DIR: Option<PathBuf> = None;
        unsafe {
            DEFAULT_LOG_DIR.get_or_insert_with(|| {
                let mut default_log_dir = exec_dir_path().to_path_buf();
                default_log_dir.push("log");
                default_log_dir
            })
        }
    }

    pub(crate) fn logger() -> &'static Logger {
        unsafe { &*std::ptr::addr_of!(GLOBAL_LOGGER) }
    }
}

#[cfg(test)]
mod tests {

    use std::hash::DefaultHasher;
    use std::hash::Hash;
    use std::hash::Hasher;
    use std::io::BufRead;
    use std::io::Cursor;
    use std::ops::AddAssign;
    use std::sync::atomic::AtomicI64;

    use chrono::Datelike;

    #[test]
    fn std_hash_test() {
        let s1 = "19900925_fliny_iiiipadjfemg";
        let s2 = "19900925_fliny_iiiipadjfemg";
        let mut s1_hasher = DefaultHasher::default();
        s1.hash(&mut s1_hasher);
        let s1_hashvalue = s1_hasher.finish();
        let mut s2_hasher = DefaultHasher::default();
        s2.hash(&mut s2_hasher);
        let s2_hashvalue = s2_hasher.finish();

        assert_eq!(s1_hashvalue, s2_hashvalue);
        println!("hash value = {}", s2_hashvalue);
    }

    #[test]
    fn chrono_test() {
        let dt = chrono::Local::now();
        let day = dt.day();
        let dat_naive = dt.date_naive();
        println!(
            "day = {}, date_naive = {}, date raw = {}",
            day, dat_naive, dt
        );
    }

    #[test]
    fn test_read_line() {
        let content = "inner content line 1\r\n\r\nline content 3";
        let mut cursor = Cursor::new(content);

        let mut line = String::new();
        let mut i = 0;

        while let Ok(size) = cursor.read_line(&mut line) {
            if size == 0 {
                break;
            }
            i += 1;
            println!("read line {} size = {}; content = {}", i, size, line);
            line.clear();
        }
        // if let Ok(size) = size_res {
        //     println!("read size = {}, content size = {}", size, content.len());
        // } else {
        //     eprintln!("{}", size_res.unwrap_err());
        // }
        // let size = reader.read_line(&mut line).unwrap();
        // println!("{:?}", line.trim().chars());
    }

    #[test]
    fn chrono_year_month_test() {
        let now = chrono::Local::now();

        assert_eq!(2024, now.year());
        assert_eq!(6, now.month());
    }

    #[test]
    fn cae_test() {
        static mut NOW: AtomicI64 = AtomicI64::new(0);
        let new_time = chrono::Local::now().timestamp();
        unsafe {
            _ = NOW.compare_exchange(
                NOW.load(std::sync::atomic::Ordering::Relaxed),
                new_time,
                std::sync::atomic::Ordering::Acquire,
                std::sync::atomic::Ordering::Relaxed,
            );
            assert_eq!(NOW.load(std::sync::atomic::Ordering::Relaxed), new_time);
        }
    }

    fn num_lock() -> &'static std::sync::RwLock<i32> {
        static mut NUM_LOCK: Option<std::sync::RwLock<i32>> = None;
        unsafe {
            match NUM_LOCK.as_ref() {
                Some(lock) => {
                    lock.write().unwrap().add_assign(400);
                    lock
                }
                None => NUM_LOCK.get_or_insert(std::sync::RwLock::new(10)),
            }
        }
    }

    #[test]
    fn tokio_blocking_write_test() {
        let r = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async { do_sth_async() });
    }

    fn do_sth_async() {
        let v = num_lock().read().unwrap();
        println!("current num = {}", *v);
        let mut v_mut = num_lock().write().unwrap();
        v_mut.add_assign(100);
        println!("after mut, num = {}", v_mut);
    }
}
