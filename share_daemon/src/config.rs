use std::{
    fs::File,
    io::Write,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
};

use toml::Table;

use crate::handler::{PathHandler, ReadAll};

pub(crate) const DEFAULT_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
pub(crate) const DEFAULT_IP_STR: &str = "0.0.0.0";
pub(crate) const DEFAULT_PORT: u16 = 0;
pub(crate) const DEFAULT_NUM_WORKERS: u8 = 3;

#[derive(serde::Serialize, Debug, Clone)]
pub struct Config {
    pub(crate) ip: IpAddr,
    pub(crate) port: u16,
    pub(crate) num_workers: u8,
    pub(crate) log_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self::new(
            SocketAddr::new(DEFAULT_IP, DEFAULT_PORT),
            0,
            PathHandler::get_default_log_dir().to_owned(),
        )
    }
}

impl Config {
    pub fn new(socket_addr: SocketAddr, num_workers: u8, log_dir: PathBuf) -> Self {
        Self {
            ip: socket_addr.ip(),
            port: socket_addr.port(),
            num_workers: if num_workers > 130 || num_workers == 0 {
                DEFAULT_NUM_WORKERS
            } else {
                num_workers
            },
            log_dir,
        }
    }

    pub fn from_socket_addr(socket_addr: SocketAddr) -> Self {
        Self::new(
            socket_addr,
            0,
            PathHandler::get_default_log_dir().to_owned(),
        )
    }
}

impl TryFrom<&Path> for Config {
    type Error = Box<dyn std::error::Error>;

    fn try_from(config_file_path: &Path) -> Result<Self, Self::Error> {
        let mut f = File::open(config_file_path)?;
        let config_str = String::from_utf8(f.read_all()?)?;
        let config_dict = config_str.parse::<Table>()?;
        let ip: IpAddr =
            if let Ok(ip_addr) = config_dict["addr"].as_str().unwrap_or(DEFAULT_IP_STR).parse() {
                ip_addr
            } else {
                DEFAULT_IP
            };
        let port: u16 = if let Ok(v) = config_dict["port"]
            .as_integer()
            .unwrap_or(DEFAULT_PORT as i64)
            .try_into()
        {
            v
        } else {
            DEFAULT_PORT
        };
        let num_workers: u8 = if let Ok(num) = config_dict["num_workers"]
            .as_integer()
            .unwrap_or(DEFAULT_NUM_WORKERS as i64)
            .try_into()
        {
            num
        } else {
            DEFAULT_NUM_WORKERS
        };
        let log_dir: PathBuf = if let Ok(log_dir_path) = config_dict["log_dir"]
            .as_str()
            .unwrap_or(PathHandler::get_default_log_dir().to_str().unwrap())
            .try_into()
        {
            log_dir_path
        } else {
            PathHandler::get_default_log_dir().to_owned()
        };

        Ok(Self {
            ip,
            port,
            num_workers,
            log_dir,
        })
    }
}

pub struct ConfigHandler {
    path: std::path::PathBuf,
    config: Config,
}

impl ConfigHandler {
    pub(crate) fn new(path: &std::path::Path) -> Self {
        Self {
            path: path.to_owned(),
            config: if let Ok(c) = Config::try_from(path) {
                c
            } else {
                Config::default()
            },
        }
    }

    pub(crate) fn generate_file(&self) {
        todo!()
    }

    pub(crate) fn save_port(&mut self, port: u16) -> std::io::Result<()> {
        let mut path = self.path.clone();
        path.push(".tinyfileshare");
        std::fs::create_dir_all(&path)?;
        path.push("port");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(&port.to_ne_bytes())?;
        f.flush()?;
        Ok(())
    }
    pub(crate) fn port(&self) -> std::io::Result<u16> {
        // read port from config file
        todo!()
    }

    pub(crate) fn num_workers(&self) -> u8 {
        // read num_workers from config file
        todo!()
    }
}
