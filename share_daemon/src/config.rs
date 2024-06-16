use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
    time::SystemTime,
};

use crate::{error::CommonError, global, CommonResult};

pub(crate) const UNSPECIFIED_PORT: u16 = 0;
pub(crate) const DEFAULT_NUM_WORKERS: u8 = 5;
pub(crate) const MAX_WORKERS: u8 = 120;
// pub(crate) const MAX_PARALLEL: u8 = 4;

#[derive(Debug, Clone)]
pub(crate) struct ConfigStore {
    current_config: Config,
    last_modified: LastModified,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum LastModified {
    LastModTime(SystemTime),
    Unknown,
}

impl ConfigStore {
    pub(crate) fn default() -> Self {
        Self {
            current_config: Config::default(),
            last_modified: LastModified::Unknown,
        }
    }

    pub(crate) fn set_config(&mut self, config: Config) {
        self.current_config = config;
    }

    pub(crate) fn from_config_file() -> Self {
        match Self::try_from_file() {
            Ok(config) => config,
            Err(e) => {
                global::logger().warn(smol_str::format_smolstr!(
                    "Error occurred while try read config from file! Sever will use default. Detail: {}",
                    e
                ));
                let mut default_config_store = Self::default();
                if let Err(e) = default_config_store.save_to_file() {
                    global::logger().error(smol_str::format_smolstr!(
                        "Error occurred while write default config to file!!! Detail: {}",
                        e
                    ));
                }
                default_config_store
            }
        }
    }

    fn try_from_file() -> CommonResult<Self> {
        let mut f = File::open(global::config_path())?;
        let mut content = vec![];
        if f.read_to_end(&mut content)? > 0 {
            let config: Config = toml::from_str(std::str::from_utf8(&content)?)?;
            let modified = if let Ok(last_modified) = f.metadata()?.modified() {
                LastModified::LastModTime(last_modified)
            } else {
                LastModified::Unknown
            };
            return Ok(Self {
                current_config: config,
                last_modified: modified,
            });
        }
        Err(CommonError::SimpleError("Config file is empty!".to_owned()))
    }

    pub(crate) fn try_update_config(&mut self) -> CommonResult<()> {
        let f = std::sync::RwLock::new(File::open(global::config_path())?);
        let mut block_f = f.write().unwrap();
        let modified_res = block_f.metadata()?.modified();
        if let Ok(last_mod_time) = modified_res {
            match self.last_modified {
                LastModified::LastModTime(time) => {
                    if time == last_mod_time {
                        return Ok(());
                    } else {
                        self.last_modified = LastModified::LastModTime(time);
                    }
                }
                _ => (),
            }
        }
        let mut bytes = vec![];
        if block_f.read_to_end(&mut bytes)? > 0 {
            let config = toml::from_str::<Config>(std::str::from_utf8(&bytes)?)?;
            self.current_config = config.checked();
        } else {
            self.save_to_file()?;
        }
        Ok(())
    }

    pub(crate) fn inner(&self) -> &Config {
        &self.current_config
    }

    pub(crate) fn mut_inner(&mut self) -> &mut Config {
        &mut self.current_config
    }

    pub(crate) fn save_to_file(&mut self) -> std::io::Result<()> {
        let f_lock = std::sync::RwLock::new(std::fs::File::create(global::config_path())?);
        let mut f_guard = f_lock.write().unwrap();
        f_guard.write_all(
            toml::to_string(&self.current_config)
                .expect("Config serialize to toml failed, this should not happen!")
                .as_bytes(),
        )?;
        f_guard.flush()?;
        if let Ok(last_modified) = f_guard.metadata()?.modified() {
            self.last_modified = LastModified::LastModTime(last_modified);
        }
        Ok(())
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct Config {
    pub(crate) listener_addr: SocketAddr,
    pub(crate) num_workers: u8,
    // trans_parallel: u8,
    pub(crate) receive_dir: PathBuf,
    reg_hosts: HashMap<String, IpAddr>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listener_addr: SocketAddr::from((Ipv4Addr::UNSPECIFIED, UNSPECIFIED_PORT)),
            num_workers: DEFAULT_NUM_WORKERS,
            receive_dir: Self::default_save_dir(),
            // trans_parallel: 0,
            reg_hosts: HashMap::new(),
        }
    }
}

impl Config {
    #[allow(unused)]
    pub(crate) fn register_host(&mut self, name: &str, ip: IpAddr) -> Option<IpAddr> {
        self.reg_hosts.insert(name.to_owned(), ip)
    }

    pub(crate) fn set_file_save_dir<P: Into<PathBuf>>(&mut self, file_save_dir: P) {
        self.receive_dir = file_save_dir.into();
    }

    pub(crate) fn set_num_workers(&mut self, n: u8) {
        self.num_workers = n;
    }

    pub(crate) fn set_listener_addr(&mut self, addr: SocketAddr) {
        self.listener_addr = addr;
    }

    // pub(crate) fn set_listener_ip(&mut self, ip_addr: IpAddr) {
    //     self.listener_addr.set_ip(ip_addr);
    // }

    pub(crate) fn set_listener_port(&mut self, port: u16) {
        self.listener_addr.set_port(port);
    }

    pub(crate) fn check_ip_registered(&self, ip: IpAddr) -> bool {
        self.reg_hosts.values().any(|reg_ip| *reg_ip == ip)
    }

    pub(crate) fn check_ip_by_name(&self, name: &str) -> Option<&IpAddr> {
        self.reg_hosts.get(name)
    }

    fn checked_num_workers(num: u8) -> u8 {
        if num == 0 || num > MAX_WORKERS {
            DEFAULT_NUM_WORKERS
        } else {
            num
        }
    }

    fn default_save_dir() -> PathBuf {
        let mut save_dir = global::home_path().to_path_buf();
        save_dir.push("tinyfileshare");
        save_dir.push("recv");
        if !save_dir.exists() {
            std::fs::create_dir_all(&save_dir)
                .expect("Unexpected: create default receive directory failed!");
        }
        return save_dir;
    }
    fn checked_log_dir(path: PathBuf) -> PathBuf {
        if !path.is_dir() {
            return global::default_log_dir().to_path_buf();
        } else {
            path
        }
    }

    // fn checked_trans_parallel(num: u8) -> u8 {
    //     if num > MAX_PARALLEL {
    //         MAX_PARALLEL
    //     } else {
    //         num
    //     }
    // }

    fn checked_save_dir(path: PathBuf) -> PathBuf {
        if path.is_dir() {
            return path;
        }
        let logger = global::logger();
        if path.is_file() || path.is_symlink() || !path.extension().is_none() {
            logger.warn("Invalid save_dir for config, use default instead.");
            return Self::default_save_dir();
        }
        if !path.exists() {
            if std::fs::create_dir_all(&path).is_err() {
                logger.warn("Create save_dir directory failed, use default instead.");
                return Self::default_save_dir();
            }
        }
        return path;
    }

    fn checked(mut self) -> Self {
        // self.log_dir = Self::checked_log_dir(self.log_dir);
        // self.trans_parallel = Self::checked_trans_parallel(self.trans_parallel);
        self.num_workers = Self::checked_num_workers(self.num_workers);
        self.receive_dir = Self::checked_save_dir(self.receive_dir);
        self
    }

    #[allow(unused)]
    pub(crate) fn new(
        addr: SocketAddr,
        num_workers: u8,
        num_parallel: u8,
        // log_dir: PathBuf,
        save_dir: PathBuf,
    ) -> Self {
        Self {
            // log_dir,
            listener_addr: addr,
            num_workers: Self::checked_num_workers(num_workers),
            // trans_parallel: Self::checked_trans_parallel(num_parallel),
            receive_dir: save_dir,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::Write, path::PathBuf};

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
            ([192, 168, 3, 2], 2082).into(),
            0,
            1,
            PathBuf::from("C:/Users/feyn/.cache/tinyfileshare/log"),
        );
        // config.add_user("feyn", "387eccc3");
        global::set_config_path(TEMP_CONF_PATH.into()).expect("Set config path failed");
        let _res = ConfigStore {
            current_config: config,
            last_modified: super::LastModified::Unknown,
        }
        .save_to_file();
    }

    #[test]
    fn deserial_test() {
        let config = Config::new(
            ([192, 168, 3, 2], 2082).into(),
            0,
            1,
            PathBuf::from("C:/Users/feyn/.cache/tinyfileshare/log"),
        );
        // config.add_user("feyn", "387eccc3");
        global::set_config_path(TEMP_CONF_PATH.into()).expect("Set config path failed");
        let read_config = ConfigStore::from_config_file();
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
