use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    net::SocketAddr,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    sync::OnceLock,
    time::SystemTime,
};

use smol_str::SmolStr;

use crate::{consts, global};

pub(crate) const DEFAULT_NUM_WORKERS: u8 = 5;
pub(crate) const MAX_WORKERS: u8 = 120;

#[derive(Debug)]
pub(crate) struct ConfigStore {
    current_config: Config,
    last_modified: LastModified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    pub(crate) fn set_config(&mut self, config: Config) -> anyhow::Result<()> {
        let (ok, c) = config.checked();
        if !ok {
            self.update_to_file()?;
        }
        self.current_config = c;
        Ok(())
    }

    pub(crate) fn from_config_file() -> Self {
        match Config::from_file(None) {
            Ok((current_config, last_modified)) => Self {
                current_config,
                last_modified,
            },
            Err(e) => {
                log::warn!(
                    "Error occurred while try read config from file! Server will use default. Detail: {}",
                    e
                );
                let mut default_config_store = Self::default();
                if let Err(e) = default_config_store.update_to_file() {
                    log::error!(
                        "Error occurred while write default config to file!!! Detail: {}",
                        e
                    );
                }

                default_config_store
            }
        }
    }

    pub(crate) fn update_to_file(&mut self) -> std::io::Result<()> {
        self.last_modified = self
            .current_config
            .write_to_file(Config::open_config_file()?)?;
        Ok(())
    }

    pub(crate) fn update_from_file(&mut self) -> anyhow::Result<()> {
        let f = Config::open_config_file()?;
        if let LastModified::LastModTime(lastmod_time) = self.last_modified {
            if let Ok(modified_time) = f.metadata()?.modified() {
                if modified_time == lastmod_time {
                    return Ok(());
                }
            }
            let (c, t) = Config::from_file(Some(f))?;
            self.current_config = c;
            self.last_modified = t;
        } else {
            self.last_modified = self.write_to_file(f)?;
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

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub(crate) listener_addr: SocketAddr,
    num_workers: u8,
    receive_dir: PathBuf,
    reg_hosts: HashMap<SmolStr, SocketAddr>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listener_addr: consts::DEFAULT_LISTENER_ADDR,
            num_workers: DEFAULT_NUM_WORKERS,
            receive_dir: Self::default_recv_dir().to_owned(),
            reg_hosts: HashMap::new(),
        }
    }
}

impl Config {
    pub(crate) fn write_to_file(&self, mut f: File) -> std::io::Result<LastModified> {
        f.write_all(
            toml::to_string(&self)
                .expect("Config serialize to toml failed, this should not happen!")
                .as_bytes(),
        )?;
        f.flush()?;
        if let Ok(last_modified) = f.metadata()?.modified() {
            return Ok(LastModified::LastModTime(last_modified));
        }
        Ok(LastModified::Unknown)
    }

    pub(crate) fn open_config_file() -> std::io::Result<File> {
        File::options()
            .read(true)
            .write(true)
            .open(global::config_path())
    }

    pub(crate) fn from_file(f_opt: Option<File>) -> anyhow::Result<(Self, LastModified)> {
        let mut f = if let Some(f) = f_opt {
            f
        } else {
            Self::open_config_file()?
        };
        let mut content = String::new();
        if f.read_to_string(&mut content)? > 0 {
            let (ok, config) = toml::from_str::<Config>(&content)?.checked();
            let modified = if !ok {
                config.write_to_file(f)?
            } else if let Ok(last_modified) = f.metadata()?.modified() {
                LastModified::LastModTime(last_modified)
            } else {
                LastModified::Unknown
            };

            return Ok((config, modified));
        }
        Err(anyhow::anyhow!("Config file is empty!"))
    }

    pub(crate) fn check_hostname(name: &str) -> anyhow::Result<&str> {
        if name.len() > consts::HOST_NAME_LENGTH_LIMIT {
            Err(anyhow::anyhow!(
                "The length of hostname {} is out of max 16(bytes)!",
                name
            ))
        } else {
            Ok(name)
        }
    }

    pub(crate) fn register_host(
        &mut self,
        hostname: &str,
        socket_addr: SocketAddr,
    ) -> Option<SocketAddr> {
        self.reg_hosts.insert(hostname.into(), socket_addr)
    }

    pub(crate) fn set_receive_dir<P: Into<PathBuf>>(&mut self, file_save_dir: P) {
        self.receive_dir = Self::checked_receive_dir(file_save_dir.into()).1;
    }

    pub(crate) fn receive_dir(&self) -> &Path {
        &self.receive_dir
    }

    pub(crate) fn num_workers(&self) -> u8 {
        self.num_workers
    }

    pub(crate) fn set_num_workers(&mut self, n: u8) {
        self.num_workers = Self::checked_num_workers(n).1;
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

    fn checked_num_workers(num: u8) -> (bool, u8) {
        if num == 0 || num > MAX_WORKERS {
            (false, DEFAULT_NUM_WORKERS)
        } else {
            (true, num)
        }
    }

    fn default_recv_dir() -> &'static Path {
        static RECV_DIR: OnceLock<PathBuf> = OnceLock::new();
        let d = RECV_DIR.get_or_init(|| dirs::download_dir().expect(consts::GET_HOME_DIR_FAILED));
        if !d.exists() {
            std::fs::create_dir_all(d)
                .expect("Unexpected: create default receive(Download) directory failed!");
        }
        d
    }

    fn checked_receive_dir(path: PathBuf) -> (bool, PathBuf) {
        if path.is_dir() {
            return (true, path);
        }
        if path.is_file() || path.is_symlink() || path.extension().is_some() {
            log::warn!("Invalid receive directory for config, use default instead.");
            return (false, Self::default_recv_dir().to_owned());
        }
        if !path.exists() && std::fs::create_dir_all(&path).is_err() {
            log::warn!("Receive directory donot exist, try create it failed, use default instead.");
            return (false, Self::default_recv_dir().to_owned());
        }
        (true, path)
    }

    fn checked(mut self) -> (bool, Self) {
        let (num_workers_ok, num_workers) = Self::checked_num_workers(self.num_workers);
        let (recv_dir_ok, recv_dir) = Self::checked_receive_dir(self.receive_dir);
        let mut hostname_ok = true;
        self.reg_hosts = self
            .reg_hosts
            .into_iter()
            .map(|(mut name, addr)| {
                if name.len() > consts::HOST_NAME_LENGTH_LIMIT {
                    if hostname_ok {
                        hostname_ok = false;
                    }
                    name = name[0..consts::HOST_NAME_LENGTH_LIMIT].into();
                }
                (name, addr)
            })
            .collect();
        let checked_ok = num_workers_ok && recv_dir_ok && hostname_ok;
        self.num_workers = num_workers;
        self.receive_dir = recv_dir;
        (checked_ok, self)
    }
}

#[cfg(test)]
mod config_tests {
    use std::net::SocketAddr;

    use crate::config::Config;

    use super::ConfigStore;

    fn get_config_store() -> ConfigStore {
        let mut config_store = ConfigStore::default();
        config_store.register_host("myhost1", SocketAddr::from(([192, 168, 3, 44], 19920)));
        config_store.register_host("myhost2", SocketAddr::from(([192, 168, 3, 121], 19920)));
        config_store.register_host(
            "myhostdngjiyhbvad",
            SocketAddr::from(([192, 179, 2, 110], 10020)),
        );
        config_store
    }

    #[test]
    fn config_to_file_test() {
        let res = get_config_store().write_to_file(Config::open_config_file().unwrap());
        assert!(res.is_ok());
    }

    #[test]
    fn config_from_file_test() {
        let res = Config::from_file(None);
        assert!(res.is_ok());
        let c = res.unwrap().0;
        assert!(c.get_addr_by_name("myhostdngjiyhbva").is_some());
    }
}
