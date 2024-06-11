pub mod request;
pub mod response;

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
    pub const NEW_LINE: &str = "\r\n";
    pub mod request {
        pub const RECV: u8 = 0x03;
        pub const SHARE: u8 = 0x01;
        pub const HOST_REG: u8 = 0x02;
        pub const PORT_PREPARE: u8 = 0x02;
        pub const TEST_REACHABLE: u8 = 0x01;
    }
    pub mod reply {
        pub const UNEXPECTED_RESPONSE: &str = "UNEXPECTED_RESPONSE";
        pub const UNREACHABLE_HOST: &str = "UNREACHABLE_HOST";
        pub const INVALID_PATHS: &str = "INVALID_PATHS";
        pub const HOST_REACHED: &str = "1";
        pub const REGISTERED_SUCCESS: &str = "REGISTER_SUCCESS";
        pub const PROGRESS: &str = "PROGRESS";
        pub const REPLACED_IP: &str = "REPLACED";
        pub const RECV_FINISHED:&str = "RECV_FINISHED";
        pub const INVALID_REQUEST: &str = "INVALID_REQUEST";
        pub const CONNECTIONS_OVERLOAD: &str = "CONNECTION_OVERLOAD";
        pub const LISTENER_STARTED: &str = "LISTENER_STARTED";
        pub const PORT_CONFIRM: &str = "PORT_CONFIRM";
        pub const ALL_PATHS_INVALID: &str = "ALL_PATHS_INVALID";
        pub const ALL_PATHS_RECEIVED: &str = "ALL_PATHS_RECEIVED";
        pub const CONNECT_HOST_FAILED: &str = "CONNECT_HOST_FAILED";
        pub const UNREGISTERED_HOST: &str = "UNREGISTERED_HOST";
    }
}

mod global {
    use std::ffi::OsStr;
    use std::path::{Path, PathBuf};
    use std::sync::OnceLock;

    use crate::error::CommonError;
    use crate::log::Logger;
    use crate::log::LoggerKind;
    use crate::CommonResult;

    // pub const BUF_SIZE: usize = 4096;
    // pub const NEWLINE: &str = "\r\n";
    pub const DEFAULT_CONFIG_DIR_NAME: &str = ".tinyfileshare";
    pub const DEFAULT_CONFIG_FILE_NAME: &str = "config.toml";
    static mut CONFIG_PATH: Option<PathBuf> = None;

    static HOME_PATH: OnceLock<PathBuf> = OnceLock::new();
    static mut GLOBAL_LOGGER: Option<Logger> = None;

    #[cfg(windows)]
    pub(crate) fn home_path() -> &'static Path {
        HOME_PATH.get_or_init(|| std::path::PathBuf::from(std::env::var("USERPROFILE").unwrap()))
    }

    #[cfg(not(windows))]
    pub(crate) fn home_path() -> std::path::PathBuf {
        HOME_PATH.get_or_init(std::path::PathBuf::from(std::env::var("HOME").unwrap()))
    }

    pub fn exec_dir_path() -> &'static Path {
        static EXE_DIR: OnceLock<PathBuf> = OnceLock::new();
        EXE_DIR.get_or_init(|| {
            let exe_path = std::env::current_exe().unwrap();
            let str_exe_path = exe_path.to_str().unwrap();
            std::path::PathBuf::from(
                &str_exe_path[0..str_exe_path.rfind(std::path::MAIN_SEPARATOR).unwrap()],
            )
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

    pub(crate) fn set_logger_kind(logger_kind: LoggerKind) {
        unsafe {
            GLOBAL_LOGGER = Some(match logger_kind {
                LoggerKind::FileLogger => Logger::file_logger(),
                LoggerKind::ConsoleLogger => Logger::console_logger(),
            })
        }
    }

    pub(crate) fn logger() -> &'static mut Logger {
        unsafe { GLOBAL_LOGGER.get_or_insert(Logger::console_logger()) }
    }
}

#[cfg(test)]
mod tests {

    use std::hash::DefaultHasher;
    use std::hash::Hash;
    use std::hash::Hasher;
    use std::io::BufRead;
    use std::io::Cursor;

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
}
