use error::CommonError;

pub mod config;
pub(crate) mod dispatcher;
pub mod error;
pub(crate) mod filedata;
pub(crate) mod handler;
pub(crate) mod log;
pub mod server;
pub(crate) mod util;
pub(crate) mod workers;

pub type CommonResult<T> = Result<T, CommonError>;

// pub type CommonResult<T> = Result<T, Box<dyn std::error::Error>>;

#[allow(unused)]
mod global {
    use crate::config::Config;
    use crate::log::LogLevel;
    use crate::log::Logger;
    use std::ffi::OsStr;
    use std::io::Stdout;
    use std::path::{Path, PathBuf};

    pub const DEFAULT_CONFIG_DIR_NAME: &str = ".tinyfileshare";
    pub const DEFAULT_CONFIG_FILE_NAME: &str = "config.toml";
    static mut CONFIG_PATH: Option<PathBuf> = None;

    fn fix_path(mut path: PathBuf) -> PathBuf {
        if path.is_file() || path.extension() == Some(OsStr::new("toml")) {
            return path;
        }
        if !path.is_dir() {
            std::fs::create_dir_all(&path).unwrap();
        }
        path.push(DEFAULT_CONFIG_FILE_NAME);
        return path;
    }

    pub(crate) fn config() -> &'static mut Config {
        static mut CONFIG: Option<Config> = None;
        unsafe { CONFIG.get_or_insert(Config::default()) }
    }

    pub(crate) fn set_config_path<P: AsRef<Path>>(path: P) {
        unsafe {
            CONFIG_PATH = Some(fix_path(path.as_ref().to_owned()));
        }
    }
    pub(crate) fn config_path() -> &'static Path {
        unsafe { CONFIG_PATH.get_or_insert(default_config_path()) }
    }

    pub(crate) fn default_config_path() -> PathBuf {
        let mut path = crate::handler::PathHandler::get_home_path();
        path.push(DEFAULT_CONFIG_DIR_NAME);
        if !path.exists() {
            std::fs::create_dir_all(&path).unwrap();
        }
        path.push(DEFAULT_CONFIG_FILE_NAME);
        path
    }

    #[cfg(feature = "file_log")]
    pub(crate) fn logger() -> &'static mut Logger<std::fs::File> {
        static mut GLOBAL_LOGGER: Option<Logger<std::fs::File>> = None;
        unsafe { GLOBAL_LOGGER.get_or_insert(Logger::new(None)) }
    }

    #[cfg(not(feature = "file_log"))]
    pub(crate) fn logger() -> &'static mut Logger<Stdout> {
        static mut GLOBAL_LOGGER: Option<Logger<Stdout>> = None;
        unsafe { GLOBAL_LOGGER.get_or_insert(Logger::new()) }
    }

    pub(crate) fn log_level() -> LogLevel {
        static mut LOG_LEVEL: &mut LogLevel = &mut LogLevel::Info;
        unsafe { *LOG_LEVEL }
    }
}

#[cfg(test)]
mod tests {

    use std::hash::DefaultHasher;
    use std::hash::Hash;
    use std::hash::Hasher;

    use chrono::Datelike;
    use sha2::Digest;
    use sha2::Sha256;

    #[test]
    fn sha2_string_test() {
        let s1 = "19900925_fliny_iiiipadjfemg".to_owned();
        let s2 = "19900925_fliny_iiiipadjfemg".to_string();
        let mut s1_hasher = Sha256::new();
        s1_hasher.update(s1);
        let s1_res = s1_hasher.finalize();
        let mut s2_hasher = Sha256::new();
        s2_hasher.update(s2);
        let s2_res = s2_hasher.finalize();
        let s1_bytes = s1_res.as_slice();
        assert_eq!(s1_bytes, &s2_res[..]);
        println!("passwd hash result = {:?}", hex::encode(s1_bytes));
    }

    #[test]
    fn std_hash_test() {
        let s1 = "19900925_fliny_iiiipadjfemg".to_owned();
        let s2 = String::from("19900925_fliny_iiiipadjfemg");
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
}
