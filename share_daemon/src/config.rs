use std::{
    ffi::OsStr,
    io::Write,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
};

use crate::{error::ServerError, global, handler::PathHandler, ServerResult};

// pub(crate) const DEFAULT_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
// pub(crate) const DEFAULT_IP_STR: &str = "0.0.0.0";
pub(crate) const UNSPECIFIED_PORT: u16 = 0;
pub(crate) const DEFAULT_NUM_WORKERS: u8 = 3;

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq)]
pub(crate) struct User {
    pub(crate) name: String,
    pub(crate) password: String,
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
    pub(crate) ip: IpAddr,
    pub(crate) port: u16,
    pub(crate) num_workers: u8,
    pub(crate) log_dir: PathBuf,
    pub(crate) users: Vec<User>,
}

impl Default for Config {
    fn default() -> Self {
        Self::new(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), UNSPECIFIED_PORT),
            0,
            PathHandler::get_default_log_dir().to_owned(),
            vec![],
        )
    }
}

impl Config {
    fn new(socket_addr: SocketAddr, num_workers: u8, log_dir: PathBuf, users: Vec<User>) -> Self {
        Self {
            ip: socket_addr.ip(),
            port: socket_addr.port(),
            num_workers: if num_workers > 130 || num_workers == 0 {
                DEFAULT_NUM_WORKERS
            } else {
                num_workers
            },
            log_dir,
            users,
        }
    }

    #[allow(unused)]
    pub(crate) fn add_user<T: Into<String>>(&mut self, name: T, password: T) {
        self.users.push(User {
            name: name.into(),
            password: password.into(),
        })
    }

    pub(crate) fn store_to_file(&self) -> ServerResult<()> {
        let mut path: PathBuf = global::config_path().to_owned();
        if !path.is_file() {
            if !path.is_dir() && path.extension() != Some(OsStr::new("toml")) {
                std::fs::create_dir_all(&path)?;
            }
            path.push(global::DEFAULT_CONFIG_FILE_NAME);
        }
        let mut f = std::fs::File::create(&path)?;
        f.write_all(
            toml::to_string(&self)
                .expect("config serialize to toml failed!")
                .as_bytes(),
        )?;
        f.flush()?;
        Ok(())
    }
}

impl TryFrom<&Path> for Config {
    type Error = ServerError;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        let mut path_buf = path.to_owned();
        if path_buf.is_dir() {
            path_buf.push(global::DEFAULT_CONFIG_FILE_NAME);
        }
        let res = std::fs::read_to_string(path_buf);
        if let Err(e) = res {
            return match e.kind() {
                std::io::ErrorKind::NotFound => Ok(Self::default()),
                _ => Err(e.into()),
            };
        }
        Ok(toml::from_str(&res.unwrap())?)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs::File,
        io::Write,
        net::{Ipv4Addr, SocketAddr, ToSocketAddrs},
        path::PathBuf,
    };

    use crate::{config, global};

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
            SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::new(192, 168, 3, 1)), 2082),
            0,
            PathBuf::from("C:/Users/feyn/.cache/tinyfileshare/log"),
            vec![("feyn", "387eccc3").into()],
        );
        // config.add_user("feyn", "387eccc3");
        global::set_config_path(TEMP_CONF_PATH);
        let res = config.store_to_file();
        assert!(res.is_ok());
    }

    #[test]
    fn deserial_test() {
        let config = Config::new(
            SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::new(192, 168, 3, 1)), 2082),
            0,
            PathBuf::from("C:/Users/feyn/.cache/tinyfileshare/log"),
            vec![("feyn", "387eccc3").into()],
        );
        // config.add_user("feyn", "387eccc3");
        global::set_config_path(TEMP_CONF_PATH);
        let read_conf_res = Config::try_from(global::config_path());
        assert!(read_conf_res.is_ok());
        assert_eq!(read_conf_res.unwrap(), config);
    }

    #[test]
    fn read_file_err_test() {
        let res = std::fs::read_to_string("C:\\Users\\feyn\\.cache\\tinyfileshare\\");
        assert_eq!(std::io::ErrorKind::NotFound, res.unwrap_err().kind())
    }
}
