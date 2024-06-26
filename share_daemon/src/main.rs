use std::{
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

use fshare_server::{request_tag::remote::PORT, server};

const DEFAULT_LOG_FILE_NAME: &str = "tinyfileshare.log";

fn check_dir_path(
    path_str: &str,
    expected_file_name: &str,
    err_msg: &'static str,
) -> anyhow::Result<PathBuf> {
    let mut path = PathBuf::from(path_str);
    if path.is_symlink() {
        return Err(anyhow::anyhow!("Symbolic link is not supported!"));
    }
    if path.is_dir() {
        path.push(expected_file_name);
        if std::fs::File::create(&path).is_ok() {
            return Ok(path);
        }
        return Err(anyhow::anyhow!(err_msg));
    }
    Err(anyhow::anyhow!(err_msg))
}

#[derive(Debug, Clone)]
struct LogDirPath(PathBuf);

impl std::str::FromStr for LogDirPath {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(check_dir_path(
            s,
            DEFAULT_LOG_FILE_NAME,
            "invalid log dir",
        )?))
    }
}

impl Deref for LogDirPath {
    type Target = PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for LogDirPath {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AsRef<Path> for LogDirPath {
    fn as_ref(&self) -> &Path {
        self.0.as_path()
    }
}

#[derive(Debug, Clone)]
struct ConfigPath(PathBuf);

impl std::fmt::Display for ConfigPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0.to_string_lossy(), f)
    }
}

impl std::str::FromStr for ConfigPath {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(path) = check_dir_path(
            s,
            fshare_server::consts::DEFAULT_CONFIG_FILE_NAME,
            "invalid config path",
        ) {
            return Ok(Self(path));
        }
        let path = Path::new(s);
        if path.is_file()
            && std::fs::File::options()
                .write(true)
                .read(true)
                .open(&path)
                .is_ok()
        {
            return match path.extension() {
                Some(ext) => {
                    if ext.to_string_lossy() == "toml" {
                        return Ok(Self(path.to_path_buf()));
                    }
                    Err(anyhow::anyhow!(
                        "Unsupported file extension: {}",
                        ext.to_string_lossy()
                    ))
                }
                None => Ok(Self(path.to_path_buf())),
            };
        }

        Err(anyhow::anyhow!("Invalid configuration path"))
    }
}

impl Deref for ConfigPath {
    type Target = PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ConfigPath {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AsRef<Path> for ConfigPath {
    fn as_ref(&self) -> &Path {
        self.0.as_path()
    }
}

mod arg_id {
    pub const CONFIG: &str = "config";
    pub const LOG_DIR: &str = "log_dir";
    pub const PORT: &str = "port";
    pub const SVR_SOCK_NAME: &str = "server_sock";
    pub const CLT_SOCK_NAME: &str = "client_sock";
    pub const LOG_LEVEL: &str = "log_level";
    pub const RECV_DIR: &str = "recv_dir";
    
}

fn main() {
    let matches = clap::Command::new(env!("CARGO_CRATE_NAME"))
        .arg(
            clap::Arg::new(arg_id::CONFIG)
                .short('c')
                .long(arg_id::CONFIG)
                .value_parser(clap::value_parser!(ConfigPath)),
        )
        .arg(
            clap::Arg::new(arg_id::LOG_DIR)
                .long(arg_id::LOG_DIR)
                .value_parser(clap::value_parser!(LogDirPath)),
        )
        .arg(clap::Arg::new(PORT))
        .get_matches();
    let mut server = fshare_server::server::Server::default();
    if let Some(config_path) = matches.get_one::<ConfigPath>(arg_id::CONFIG) {}
    if let Err(e) = server::Server::default().start() {
        eprintln!("Start server failed: {}", e);
    }
}
