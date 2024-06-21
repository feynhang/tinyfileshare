use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    net::SocketAddr,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    time::SystemTime,
};

use smol_str::ToSmolStr;

use crate::{consts, global};

pub(crate) const DEFAULT_NUM_WORKERS: u8 = 5;
pub(crate) const MAX_WORKERS: u8 = 120;
// pub(crate) const MAX_PARALLEL: u8 = 4;

#[derive(Debug)]
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

    pub(crate) fn clone_inner(&self) -> Config {
        self.current_config.clone()
    }

    pub(crate) fn set_config(&mut self, config: Config) {
        self.current_config = config;
    }

    pub(crate) fn from_config_file() -> Self {
        match Self::try_from_file() {
            Ok(config) => config,
            Err(e) => {
                log::warn!(
                    "Error occurred while try read config from file! Server will use default. Detail: {}",
                    e
                );
                let mut default_config_store = Self::default();
                if let Err(e) = default_config_store.save_to_file() {
                    log::error!(
                        "Error occurred while write default config to file!!! Detail: {}",
                        e
                    );
                }
                default_config_store
            }
        }
    }

    fn try_from_file() -> anyhow::Result<Self> {
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
        Err(anyhow::Error::msg("Config file is empty!".to_smolstr()))
    }

    pub(crate) fn try_update_config(&mut self) -> anyhow::Result<()> {
        let mut f = File::open(global::config_path())?;
        let modified_res = f.metadata()?.modified();
        if let Ok(last_mod_time) = modified_res {
            if let LastModified::LastModTime(time) = self.last_modified {
                if time == last_mod_time {
                    return Ok(());
                } else {
                    self.last_modified = LastModified::LastModTime(time);
                }
            }
        }
        let mut bytes = vec![];
        if f.read_to_end(&mut bytes)? > 0 {
            let config = toml::from_str::<Config>(std::str::from_utf8(&bytes)?)?;
            self.current_config = config.checked();
        } else {
            self.save_to_file()?;
        }
        Ok(())
    }

    pub(crate) fn save_to_file(&mut self) -> std::io::Result<()> {
        let mut f = std::fs::File::create(global::config_path())?;
        f.write_all(
            toml::to_string(&self.current_config)
                .expect("Config serialize to toml failed, this should not happen!")
                .as_bytes(),
        )?;
        f.flush()?;
        if let Ok(last_modified) = f.metadata()?.modified() {
            self.last_modified = LastModified::LastModTime(last_modified);
        }
        Ok(())
    }
}

impl Deref for ConfigStore {
    type Target = Config;

    fn deref(&self) -> &Self::Target {
        &self.current_config
    }
}

impl DerefMut for ConfigStore {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.current_config
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct Config {
    pub(crate) listener_addr: SocketAddr,
    num_workers: u8,
    receive_dir: PathBuf,
    reg_hosts: HashMap<String, SocketAddr>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listener_addr: consts::DEFAULT_LISTENER_ADDR,
            num_workers: DEFAULT_NUM_WORKERS,
            receive_dir: Self::default_recv_dir(),
            reg_hosts: HashMap::new(),
        }
    }
}

impl Config {
    #[allow(unused)]
    pub(crate) fn register_host(&mut self, name: &str, ip: SocketAddr) -> Option<SocketAddr> {
        self.reg_hosts.insert(name.to_owned(), ip)
    }

    pub(crate) fn set_receive_dir<P: Into<PathBuf>>(&mut self, file_save_dir: P) {
        self.receive_dir = Self::checked_receive_dir(file_save_dir.into());
    }

    pub(crate) fn receive_dir(&self) -> &Path {
        &self.receive_dir
    }

    pub(crate) fn num_workers(&self) -> u8 {
        self.num_workers
    }

    pub(crate) fn set_num_workers(&mut self, n: u8) {
        self.num_workers = Self::checked_num_workers(n);
    }

    pub(crate) fn set_listener_addr(&mut self, addr: SocketAddr) {
        self.listener_addr = addr;
    }

    pub(crate) fn set_listener_port(&mut self, port: u16) {
        self.listener_addr.set_port(port);
    }

    pub(crate) fn check_addr_registered(&self, addr: SocketAddr) -> bool {
        self.reg_hosts.values().any(|reg_ip| *reg_ip == addr)
    }

    pub(crate) fn get_addr_by_name(&self, hostname: &str) -> Option<&SocketAddr> {
        self.reg_hosts.get(hostname)
    }

    fn checked_num_workers(num: u8) -> u8 {
        if num == 0 || num > MAX_WORKERS {
            DEFAULT_NUM_WORKERS
        } else {
            num
        }
    }

    fn default_recv_dir() -> PathBuf {
        let receive_dir = dirs::download_dir().expect(consts::GET_HOME_DIR_FAILED);
        if !receive_dir.exists() {
            std::fs::create_dir_all(&receive_dir)
                .expect("Unexpected: create default receive(Download) directory failed!");
        }
        receive_dir
    }

    fn checked_receive_dir(path: PathBuf) -> PathBuf {
        if path.is_dir() {
            return path;
        }
        if path.is_file() || path.is_symlink() || path.extension().is_some() {
            log::warn!("Invalid receive directory for config, use default instead.");
            return Self::default_recv_dir();
        }
        if !path.exists() && std::fs::create_dir_all(&path).is_err() {
            log::warn!("Receive directory donot exist, try create it failed, use default instead.");
            return Self::default_recv_dir();
        }
        path
    }

    fn checked(mut self) -> Self {
        self.num_workers = Self::checked_num_workers(self.num_workers);
        self.receive_dir = Self::checked_receive_dir(self.receive_dir);
        self
    }
}
