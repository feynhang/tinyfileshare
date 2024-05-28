use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    io::Write,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

use crate::{global, handler::PathHandler, CommonResult};

// pub(crate) const DEFAULT_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
// pub(crate) const DEFAULT_IP_STR: &str = "0.0.0.0";
pub(crate) const UNSPECIFIED_PORT: u16 = 0;
pub(crate) const DEFAULT_NUM_WORKERS: u8 = 3;

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub(crate) struct User {
    pub(crate) name: String,
    pub(crate) password: String,
}

impl User {
    pub fn new<T: AsRef<str>>(name: T, password: T) -> Self {
        Self {
            name: name.as_ref().to_owned(),
            password: password.as_ref().to_owned(),
        }
    }

    pub fn check_password<T: AsRef<str>>(&self, password: T) -> bool {
        let mut hasher = Sha256::new();
        hasher.update(password.as_ref());
        self.password == hex::encode(hasher.finalize().as_slice())
    }
}

impl PartialEq for User {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl<T> From<(T, T)> for User
where
    T: Into<String>,
{
    fn from((name, password): (T, T)) -> Self {
        Self {
            name: name.into(),
            password: password.into(),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct Config {
    log_path: PathBuf,
    addr: SocketAddr,
    num_workers: u8,
    users_dict: HashMap<u64, User>,
}

impl Default for Config {
    fn default() -> Self {
        let mut log_path = PathHandler::get_default_log_dir().to_owned();
        log_path.push(format!("{}.log", chrono::Local::now().date_naive()));
        Self {
            addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), UNSPECIFIED_PORT),
            num_workers: DEFAULT_NUM_WORKERS,
            log_path,
            users_dict: HashMap::new(),
        }
    }
}

impl Config {
    fn simple_hash(value: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        return hasher.finish();
    }

    pub(crate) fn socket_addr(&self) -> SocketAddr {
        self.addr
    }

    pub(crate) fn num_workers(&self) -> u8 {
        self.num_workers
    }

    pub(crate) fn set_addr<A: Into<SocketAddr>>(&mut self, addr: A) {
        self.addr = addr.into()
    }

    pub(crate) fn check_user(&self, name: &str) -> bool {
        let v = Self::simple_hash(name);
        self.users_dict.contains_key(&v)
    }

    pub(crate) fn from_file(path: &Path) -> Self {
        let config_file_path = if !path.exists() {
            let p = global::default_config_path();
            global::logger().log(
                "The specified config file not exist,try default config file.",
                crate::log::LogLevel::Warn,
            );
            if !p.exists() {
                global::logger().log(
                    "Default config file not exist, create default config.",
                    crate::log::LogLevel::Warn,
                );
                return Self::default();
            }
            p
        } else {
            path.to_owned()
        };
        let res = std::fs::read_to_string(config_file_path);
        if let Err(e) = res {
            global::logger().log(
                format!(
                    "Read config file failed, create default config. Detail: {}",
                    e
                ),
                crate::log::LogLevel::Warn,
            );
            return Self::default();
        }
        let des_res = toml::from_str(&res.unwrap());
        if let Err(e) = des_res {
            global::logger().log(
                format!(
                    "Deserialize config file content failed, create default config. Detail: {}",
                    e
                ),
                crate::log::LogLevel::Warn,
            );
            return Self::default();
        }
        des_res.unwrap()
    }

    #[allow(unused)]
    pub(crate) fn add_user<T: Into<String>>(&mut self, name: T, password: T) {
        let name_str = name.into();
        self.users_dict.insert(
            Self::simple_hash(&name_str),
            User {
                name: name_str,
                password: password.into(),
            },
        );
    }

    pub(crate) fn store(&self) -> CommonResult<()> {
        let mut f = std::fs::File::create(global::config_path())?;
        f.write_all(
            toml::to_string(&self)
                .expect("config serialize to toml failed!")
                .as_bytes(),
        )?;
        f.flush()?;
        Ok(())
    }

    pub(crate) fn new(
        ip: IpAddr,
        port: u16,
        mut num_workers: u8,
        log_dir: PathBuf,
        users: Vec<User>,
    ) -> Self {
        num_workers = if num_workers == 0 || num_workers > 120 {
            3
        } else {
            num_workers
        };
        let mut users_dict = HashMap::new();
        for u in users {
            users_dict.insert(Self::simple_hash(&u.name), u);
        }
        Self {
            log_path: log_dir,
            addr: SocketAddr::new(ip, port),
            num_workers,
            users_dict,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::Write, net::Ipv4Addr, path::PathBuf};

    use crate::global;

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
            std::net::IpAddr::V4(Ipv4Addr::new(192, 168, 3, 1)),
            2082,
            0,
            PathBuf::from("C:/Users/feyn/.cache/tinyfileshare/log"),
            vec![("feyn", "387eccc3").into()],
        );
        // config.add_user("feyn", "387eccc3");
        global::set_config_path(TEMP_CONF_PATH);
        let res = config.store();
        assert!(res.is_ok());
    }

    #[test]
    fn deserial_test() {
        let config = Config::new(
            std::net::IpAddr::V4(Ipv4Addr::new(192, 168, 3, 1)),
            2082,
            0,
            PathBuf::from("C:/Users/feyn/.cache/tinyfileshare/log"),
            vec![("feyn", "387eccc3").into()],
        );
        // config.add_user("feyn", "387eccc3");
        global::set_config_path(TEMP_CONF_PATH);
        let read_config = Config::from_file(global::config_path());
        assert_eq!(read_config, config);
    }

    #[test]
    fn read_file_err_test() {
        let res = std::fs::read_to_string("C:\\Users\\feyn\\.cache\\tinyfileshare\\");
        assert_eq!(std::io::ErrorKind::NotFound, res.unwrap_err().kind())
    }
}
