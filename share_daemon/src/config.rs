use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    net::IpAddr,
    path::{Path, PathBuf},
    time::SystemTime,
};

use crate::global;

pub(crate) const UNSPECIFIED_PORT: u16 = 0;
pub(crate) const DEFAULT_NUM_WORKERS: u8 = 5;
pub(crate) const MAX_WORKERS: u8 = 120;
pub(crate) const MAX_PARALLEL: u8 = 4;

pub(crate) const DEFAULT_CLIENT_PORT: u16 = 10020;

pub(crate) struct ConfigStore {
    current_config: Config,
    last_modified: LastModified,
}

#[derive(Debug)]
pub(crate) enum LastModified {
    Unsaved,
    LastModTime(SystemTime),
    Unsupported,
}

impl PartialEq for LastModified {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::LastModTime(l0), Self::LastModTime(r0)) => *l0 == *r0,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl ConfigStore {
    fn default() -> Self {
        Self {
            current_config: Config::default(),
            last_modified: LastModified::Unsaved,
        }
    }

    pub(crate) fn check_ip_registered(&self, ip: IpAddr) -> bool {
        self
        .current_config
        .reg_hosts
        .values()
        .any(|reg_host| *reg_host == ip)
    }
    pub(crate) fn register_host(&mut self, name: &str, ip: IpAddr) -> Option<IpAddr> {
        self.last_modified = LastModified::Unsaved;
        self.current_config.reg_hosts.insert(name.to_owned(), ip)
    }

    pub(crate) fn get_host_by_name(&self, name: &str) -> Option<&IpAddr> {
        self.current_config.reg_hosts.get(name)
    }

    pub(crate) fn number_of_workers(&self) -> u8 {
        self.current_config.num_workers
    }

    pub(crate) fn set_start_port(&mut self, port: u16) {
        self.last_modified = LastModified::Unsaved;
        self.current_config.start_port = port;
    }

    pub(crate) fn log_dir(&self) -> &Path {
        &self.current_config.log_dir
    }

    pub(crate) fn use_default(&mut self) {
        self.current_config = Default::default();
        self.last_modified = LastModified::Unsaved;
    }

    pub(crate) fn start_port(&self) -> u16 {
        self.current_config.start_port
    }

    pub(crate) fn from_config_path() -> Self {
        let logger = global::logger();

        if let Ok(mut f) = File::open(global::config_path()) {
            let last_modified = if let Ok(modified) = f.metadata().unwrap().modified() {
                LastModified::LastModTime(modified)
            } else {
                logger.info(
                    "Last modified time is not supported, config will always read from file.",
                );
                LastModified::Unsupported
            };
            let mut content = vec![];
            if let Ok(content_size) = f.read_to_end(&mut content) {
                if content_size != 0 {
                    if let Ok(content_str) = std::str::from_utf8(&content) {
                        if let Ok(config) = toml::from_str(content_str) {
                            return Self {
                                current_config: config,
                                last_modified,
                            };
                        } else {
                            logger.info("Deserialize config from file failed!");
                        }
                    } else {
                        logger.info("Config file has non-UTF-8 content!");
                    }
                } else {
                    logger.info("Empty config file! Create default.");
                }
            } else {
                logger.info("Read config file failed, use default config instead.");
            }
        } else {
            logger.info(format_args!(
                "Open config file from path '{}' failed, use default config instead.",
                global::config_path().to_string_lossy()
            ));
        }
        return Self::default();
    }

    pub(crate) fn update_config(&mut self) {
        if let Ok(mut f) = File::open(global::config_path()) {
            let modified_res = f.metadata().unwrap().modified();
            if let Ok(last_mod_time) = modified_res {
                match self.last_modified {
                    LastModified::LastModTime(time) if last_mod_time == time => return,
                    LastModified::LastModTime(t) => {
                        self.last_modified = LastModified::LastModTime(t)
                    }
                    LastModified::Unsaved => {
                        self.save_to_file();
                        return;
                    },
                    _ => (),
                }
            }
            let mut bytes = vec![];
            if let Ok(size) = f.read_to_end(&mut bytes) {
                if size != 0 {
                    if let Ok(config_str) = std::str::from_utf8(&bytes) {
                        if let Ok(config) = toml::from_str::<Config>(config_str) {
                            self.current_config = config.checked();
                            return;
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn inner_config(&mut self) -> &mut Config {
        &mut self.current_config
    }

    fn save_to_file(&mut self) {
        let mut f = std::fs::File::create(global::config_path())
            .expect("Create or open config file failed, this should not happen!");
        f.write_all(
            toml::to_string(&self.current_config)
                .expect("Config serialize to toml failed, this should not happen!")
                .as_bytes(),
        )
        .expect("Unexpeced: write serialized config data to file failed!");
        if let Ok(last_modified) = f.metadata().unwrap().modified() {
            self.last_modified = LastModified::LastModTime(last_modified);
        } else {
            self.last_modified = LastModified::Unsupported;
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct Config {
    log_dir: PathBuf,
    start_port: u16,
    num_workers: u8,
    /// Number of parallel connections to receive files
    trans_parallel: u8,
    recv_dir: PathBuf,
    reg_hosts: HashMap<String, IpAddr>,
}

impl Default for Config {
    fn default() -> Self {
        // log_dir.push(format!("{}.log", chrono::Local::now().date_naive()));
        
        Self {
            start_port: UNSPECIFIED_PORT,
            num_workers: DEFAULT_NUM_WORKERS,
            log_dir: global::default_log_dir().to_owned(),
            recv_dir: Self::default_recv_dir(),
            trans_parallel: 0,
            reg_hosts: HashMap::new(),
        }
    }
}

impl Config {
    fn checked_num_workers(num: u8) -> u8 {
        if num == 0 || num > MAX_WORKERS {
            DEFAULT_NUM_WORKERS
        } else {
            num
        }
    }

    fn default_recv_dir() -> PathBuf {
        let mut recv_dir = global::home_path().to_path_buf();
        recv_dir.push("tinyfileshare");
        recv_dir.push("recv");
        if !recv_dir.exists() {
            std::fs::create_dir_all(&recv_dir).expect("Unexpected: create default receive directory failed!");
        }
        return recv_dir;
    }
    fn checked_log_dir(path: PathBuf) -> PathBuf {
        if !path.is_dir() {
            return global::default_log_dir().to_path_buf();
        } else {
            path
        }
    }

    fn checked_trans_parallel(num: u8) -> u8 {
        if num > MAX_PARALLEL {
            MAX_PARALLEL
        } else {
            num
        }
    }

    fn checked_recv_dir(mut path: PathBuf) -> PathBuf {
        if path.is_dir() {
            return path;
        }
        let logger = global::logger();
        if path.is_file() || path.is_symlink() || !path.extension().is_none() {
            logger.warn("Invalid recv_dir for config, use default instead.");
            return Self::default_recv_dir();
        }
        if !path.exists() {
            if std::fs::create_dir_all(&path).is_err() {
                logger.warn("Create recv_dir directory failed, use default instead.");
                return Self::default_recv_dir();
            }
        }
        return path;
    }

    fn checked(mut self) -> Self {
        self.log_dir = Self::checked_log_dir(self.log_dir);
        self.trans_parallel = Self::checked_trans_parallel(self.trans_parallel);
        self.num_workers = Self::checked_num_workers(self.num_workers);
        self.recv_dir = Self::checked_recv_dir(self.recv_dir);
        self
    }

    pub(crate) fn new(
        port: u16,
        num_workers: u8,
        num_parallel: u8,
        log_dir: PathBuf,
        recv_dir: PathBuf,
    ) -> Self {
        Self {
            log_dir,
            start_port: port,
            num_workers: Self::checked_num_workers(num_workers),
            trans_parallel: Self::checked_trans_parallel(num_parallel),
            recv_dir,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::Write, net::Ipv4Addr, path::PathBuf};

    use crate::{config::ConfigStore, global};

    use super::Config;

    #[test]
    fn create_dir_all_test() {
        let mut path = PathBuf::from("C:\\Users\\feyn\\.cache\\from_tinyfileshare\\temp");

        std::fs::create_dir_all(&path).unwrap();
        path.push("config.toml");
        File::create(&path)
            .unwrap()
            .write_all("temp contents".as_bytes())
            .unwrap()
    }

    const TEMP_CONF_PATH: &str = "C:\\Users\\feyn\\.cache\\tinyfileshare\\configdir";

    #[test]
    fn serial_test() {
        let config = Config::new(
            2082,
            0,
            1,
            PathBuf::from("C:/Users/feyn/.cache/tinyfileshare/log"),
            PathBuf::from("C:/Users/feyn/.cache/tinyfileshare_recv"),
        );
        // config.add_user("feyn", "387eccc3");
        global::set_config_path(TEMP_CONF_PATH.into()).expect("Set config path failed");
        let _res = ConfigStore {
            current_config: config,
            last_modified: super::LastModified::Unsaved,
        }
        .save_to_file();
    }

    #[test]
    fn deserial_test() {
        let config = Config::new(
            2082,
            0,
            1,
            PathBuf::from("C:/Users/feyn/.cache/tinyfileshare/log"),
            PathBuf::from("C:/Users/feyn/.cache/tinyfileshare_recv"),
        );
        // config.add_user("feyn", "387eccc3");
        global::set_config_path(TEMP_CONF_PATH.into()).expect("Set config path failed");
        let read_config = ConfigStore::from_config_path();
        assert_eq!(read_config.current_config, config);
    }

    #[test]
    fn read_file_err_test() {
        let res = std::fs::read_to_string("C:\\Users\\feyn\\.cache\\tinyfileshare\\");
        assert_eq!(std::io::ErrorKind::NotFound, res.unwrap_err().kind())
    }

    #[test]
    fn write_last_test() {
        let mut f = File::options()
            .append(true)
            .read(true)
            .open("C:/Users/feyn/.cache/tinyfileshare/temp.txt")
            .unwrap();
        let modified_before = f.metadata().unwrap().modified().unwrap();
        f.write_all(b"\r\nnew content\r\n").unwrap();
        // f.flush().unwrap();
        let modified_after = f.metadata().unwrap().modified().unwrap();
        assert_ne!(modified_after, modified_before);
    }
}
