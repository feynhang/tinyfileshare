use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    net::{IpAddr, SocketAddr},
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    sync::OnceLock,
    time::SystemTime,
};

use smol_str::SmolStr;

use crate::consts;
pub(crate) const GET_HOME_DIR_FAILED: &str =
    "Unexpected: get home dir failed! Maybe you are in an unsupported platform!";

pub(crate) const MAX_WORKERS: u8 = 120;

#[derive(Debug)]
pub(crate) struct ConfigStore {
    current_config: Config,
    config_path: PathBuf,
    last_modified: LastModified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LastModified {
    LastModTime(SystemTime),
    Unsupported,
    Unsaved,
}

impl ConfigStore {
    pub(crate) fn default() -> Self {
        Self {
            current_config: Config::default(),
            config_path: Config::default_config_path(),
            last_modified: LastModified::Unsaved,
        }
    }

    pub(crate) fn set_config(&mut self, config: Config) {
        self.current_config = config;
    }

    pub(crate) fn update_to_file(&mut self) -> anyhow::Result<()> {
        self.last_modified = self.write_to_file(&self.config_path)?;
        Ok(())
    }

    pub(crate) fn try_update_from_file(&mut self) -> anyhow::Result<()> {
        let f = Config::open_config_file_readonly(&self.config_path)?;
        match self.last_modified {
            LastModified::LastModTime(last_mod_time) => {
                if let Ok(mod_time) = f.metadata()?.modified() {
                    if mod_time == last_mod_time {
                        return Ok(());
                    }
                }
            }
            LastModified::Unsaved => {
                self.last_modified = self.current_config.write_to_file(&self.config_path)?;
                return Ok(());
            }
            _ => (),
        }
        self.last_modified = self.current_config.update_from(&self.config_path)?;
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
    listener_addr: SocketAddr,
    num_workers: u8,
    save_dir: PathBuf,
    ipc_socket_name: SmolStr,
    reg_hosts: HashMap<SmolStr, SocketAddr>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listener_addr: consts::DEFAULT_LISTENER_ADDR,
            num_workers: consts::DEFAULT_NUM_WORKERS,
            save_dir: Self::default_save_dir().to_owned(),
            ipc_socket_name: consts::DEFAULT_IPC_SOCK_NAME.into(),
            reg_hosts: HashMap::new(),
        }
    }
}

impl Config {
    // immutable self

    pub(crate) fn write_to_file(&self, p: &Path) -> anyhow::Result<LastModified> {
        let mut f = File::create(p)?;
        f.write_all(toml::to_string(&self)?.as_bytes())?;
        f.flush()?;
        if let Ok(last_modified) = f.metadata()?.modified() {
            return Ok(LastModified::LastModTime(last_modified));
        }
        Ok(LastModified::Unsupported)
    }

    pub(crate) fn ipc_socket_name(&self) -> &str {
        &self.ipc_socket_name
    }

    pub(crate) fn check_addr_registered(&self, addr: SocketAddr) -> bool {
        self.reg_hosts.values().any(|reg_ip| *reg_ip == addr)
    }

    pub(crate) fn get_addr_by_name(&self, hostname: &str) -> Option<&SocketAddr> {
        self.reg_hosts.get(hostname)
    }

    pub(crate) fn listener_addr(&self) -> SocketAddr {
        self.listener_addr
    }

    pub(crate) fn receive_dir(&self) -> &Path {
        &self.save_dir
    }

    pub(crate) fn num_workers(&self) -> u8 {
        self.num_workers
    }

    // mutable self

    pub(crate) fn update_from<P: AsRef<Path>>(&mut self, path: P) -> anyhow::Result<LastModified> {
        let p = path.as_ref();
        let (c, t) = Config::from_file(p)?;
        let (mut config_ok, mut config) = c.checked();
        config_ok = config_ok && config.listener_addr == self.listener_addr;
        Ok(if !config_ok {
            config.listener_addr = self.listener_addr;
            *self = config;
            self.write_to_file(p)?
        } else {
            t
        })
    }

    pub(crate) fn register_host(
        &mut self,
        hostname: &str,
        socket_addr: SocketAddr,
    ) -> Option<SocketAddr> {
        self.reg_hosts.insert(hostname.into(), socket_addr)
    }

    pub(crate) fn set_save_dir<P: Into<PathBuf>>(&mut self, files_save_dir: P) {
        self.save_dir = Self::check_files_save_dir(files_save_dir.into()).1;
    }

    pub(crate) fn set_num_workers(&mut self, n: u8) {
        self.num_workers = Self::check_num_workers(n).1;
    }

    pub(crate) fn set_listener_addr(&mut self, addr: SocketAddr) {
        self.listener_addr = addr;
    }

    pub(crate) fn set_listener_port(&mut self, port: u16) {
        self.listener_addr.set_port(port);
    }

    pub(crate) fn set_ipc_socket_name(&mut self, name: SmolStr) {
        self.ipc_socket_name = name;
    }

    pub(crate) fn set_listener_ip(&mut self, ip: IpAddr) {
        self.listener_addr.set_ip(ip)
    }

    // static

    pub(crate) fn default_config_path() -> PathBuf {
        let mut path = dirs::home_dir().expect(GET_HOME_DIR_FAILED);
        path.push(consts::DEFAULT_CONFIG_DIR_NAME);
        if !path.exists() {
            std::fs::create_dir_all(&path).unwrap();
        }
        path.push(consts::DEFAULT_CONFIG_FILE_NAME);
        path
    }

    pub(crate) fn open_config_file_readonly<P: AsRef<Path>>(
        config_path: P,
    ) -> std::io::Result<File> {
        File::open(config_path)
    }

    pub(crate) fn from_file<P: AsRef<Path>>(p: P) -> anyhow::Result<(Self, LastModified)> {
        let mut f = File::open(p.as_ref())?;
        let mut content = String::new();
        if f.read_to_string(&mut content)? > 0 {
            let (ok, config) = toml::from_str::<Config>(content.trim())?.checked();
            let modified = if !ok {
                config.write_to_file(p.as_ref())?
            } else if let Ok(last_modified) = f.metadata()?.modified() {
                LastModified::LastModTime(last_modified)
            } else {
                LastModified::Unsupported
            };

            Ok((config, modified))
        } else {
            Err(anyhow::anyhow!("Config file is empty!"))
        }
    }

    fn check_num_workers(num: u8) -> (bool, u8) {
        if num == 0 || num > MAX_WORKERS {
            (false, consts::DEFAULT_NUM_WORKERS)
        } else {
            (true, num)
        }
    }

    fn default_save_dir() -> &'static Path {
        static RECV_DIR: OnceLock<PathBuf> = OnceLock::new();
        let d = RECV_DIR.get_or_init(|| dirs::download_dir().expect(GET_HOME_DIR_FAILED));
        if !d.exists() {
            std::fs::create_dir_all(d)
                .expect("Unexpected: create default receive(Download) directory failed!");
        }
        d
    }

    fn check_files_save_dir(path: PathBuf) -> (bool, PathBuf) {
        if !path.is_dir() {
            log::warn!("Invalid files save directory! using default instead.");
            return (false, Self::default_save_dir().to_owned());
        }
        (true, path)
    }

    #[inline(always)]
    pub(crate) fn check_hostname_valid(hostname: &str) -> bool {
        let len = hostname.len();
        len > 0 && len <= consts::HOST_NAME_LENGTH_LIMIT
    }

    fn check_addr_valid(addr: SocketAddr) -> bool {
        std::net::TcpStream::connect(addr).is_ok()
    }

    pub(crate) fn checked(mut self) -> (bool, Self) {
        let (num_workers_ok, num_workers) = Self::check_num_workers(self.num_workers);
        let (recv_dir_ok, recv_dir) = Self::check_files_save_dir(self.save_dir);
        let hosts_count = self.reg_hosts.len();
        self.reg_hosts
            .retain(|name, addr| Self::check_hostname_valid(name) && Self::check_addr_valid(*addr));
        let checked_ok = num_workers_ok && recv_dir_ok && self.reg_hosts.len() == hosts_count;
        self.num_workers = num_workers;
        self.save_dir = recv_dir;
        (checked_ok, self)
    }
}
