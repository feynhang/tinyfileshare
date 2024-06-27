#![allow(unused)]
use std::{
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

use faccess::{AccessMode, PathExt};
use fshare_server::server;
use smol_str::{SmolStr, StrExt, ToSmolStr};

#[inline(always)]
fn check_symlink(p: &Path) -> anyhow::Result<()> {
    if p.is_symlink() {
        return Err(anyhow::anyhow!("Symbolic link is not supported!"));
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct DirPath(PathBuf);

impl std::str::FromStr for DirPath {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let path = Path::new(s);
        check_symlink(path)?;
        if path.is_dir() && path.access(AccessMode::READ | AccessMode::WRITE).is_ok() {
            return Ok(Self(path.to_path_buf()));
        }
        Err(anyhow::anyhow!("Invalid directory path!"))
    }
}

impl Deref for DirPath {
    type Target = PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DirPath {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
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
        let path = Path::new(s);
        check_symlink(path)?;
        let ext_name = path.extension().map(|os_str_ext| os_str_ext.to_str());
        if path.is_file()
            && matches!(ext_name, Some(Some("toml")) | None)
            && std::fs::File::options()
                .write(true)
                .read(true)
                .open(path)
                .is_ok()
        {
            return Ok(Self(path.to_path_buf()));
        }
        Err(anyhow::anyhow!("Invalid configuration file path"))
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

#[derive(Debug, Clone)]
struct SockName(SmolStr);

impl std::str::FromStr for SockName {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        static mut NAME_PREVIOUS: Option<SmolStr> = None;
        if s.is_empty() {
            return Err(anyhow::anyhow!("socket name cannot be empty!"));
        }
        let n = unsafe {
            match NAME_PREVIOUS.as_ref() {
                Some(name_before) => {
                    if name_before == s {
                        return Err(anyhow::anyhow!(
                            "Server socket name cannot be same as client socket name!"
                        ));
                    }
                    s.to_smolstr()
                }
                None => {
                    let n = s.to_smolstr();
                    NAME_PREVIOUS = Some(n.clone());
                    n
                }
            }
        };
        Ok(Self(n))
    }
}

impl Deref for SockName {
    type Target = SmolStr;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Copy)]
enum LogLevel {
    /// A level lower than all log levels.
    Off,
    /// Corresponds to the `Error` log level.
    Error,
    /// Corresponds to the `Warn` log level.
    Warn,
    /// Corresponds to the `Info` log level.
    Info,
    /// Corresponds to the `Debug` log level.
    Debug,
    /// Corresponds to the `Trace` log level.
    Trace,
}

impl LogLevel {
    pub fn to_filter(self) -> log::LevelFilter {
        self.into()
    }
}

impl From<LogLevel> for log::LevelFilter {
    fn from(value: LogLevel) -> Self {
        match value {
            LogLevel::Off => log::LevelFilter::Off,
            LogLevel::Error => log::LevelFilter::Error,
            LogLevel::Warn => log::LevelFilter::Warn,
            LogLevel::Info => log::LevelFilter::Info,
            LogLevel::Debug => log::LevelFilter::Debug,
            LogLevel::Trace => log::LevelFilter::Trace,
        }
    }
}

impl std::str::FromStr for LogLevel {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() || s.len() > 5 {
            return Err(anyhow::anyhow!("Invalid log level!"));
        }
        let l = s.to_ascii_lowercase_smolstr();
        match l.as_str() {
            "off" => Ok(Self::Off),
            "error" => Ok(Self::Error),
            "warn" => Ok(Self::Warn),
            "info" => Ok(Self::Info),
            "debug" => Ok(Self::Debug),
            "trace" => Ok(Self::Trace),
            _ => Err(anyhow::anyhow!("Invalid log level!")),
        }
    }
}

impl clap::ValueEnum for LogLevel {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            Self::Off,
            Self::Error,
            Self::Warn,
            Self::Info,
            Self::Debug,
            Self::Trace,
        ]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(clap::builder::PossibleValue::new(match self {
            LogLevel::Off => "off",
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
        }))
    }
}

mod arg_id {
    pub const CONFIG: &str = "config";
    pub const LOG_DIR: &str = "log_dir";
    pub const PORT: &str = "port";
    pub const SVR_SOCK_NAME: &str = "server_socket";
    pub const CLT_SOCK_NAME: &str = "client_socket";
    pub const LOG_LEVEL: &str = "log_level";
    pub const DEFAULT_SAVE_DIR: &str = "save_dir";
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
                .value_parser(clap::value_parser!(DirPath)),
        )
        .arg(
            clap::Arg::new(arg_id::PORT)
                .short('p')
                .long(arg_id::PORT)
                .value_parser(clap::value_parser!(u16).range(3000..)),
        )
        .arg(
            clap::Arg::new(arg_id::SVR_SOCK_NAME)
                .long(arg_id::SVR_SOCK_NAME)
                .value_parser(clap::value_parser!(SockName)),
        )
        .arg(
            clap::Arg::new(arg_id::CLT_SOCK_NAME)
                .long(arg_id::CLT_SOCK_NAME)
                .value_parser(clap::value_parser!(SockName)),
        )
        .arg(
            clap::Arg::new(arg_id::LOG_LEVEL)
                .long(arg_id::LOG_LEVEL)
                .value_parser(clap::builder::EnumValueParser::<LogLevel>::new())
                .ignore_case(true),
        )
        .arg(
            clap::Arg::new(arg_id::DEFAULT_SAVE_DIR)
                .long(arg_id::DEFAULT_SAVE_DIR)
                .value_parser(clap::value_parser!(DirPath)),
        )
        .get_matches();
    let mut server = fshare_server::server::Server::default();
    if let Some(config_path) = matches.get_one::<ConfigPath>(arg_id::CONFIG) {}
    if let Err(e) = server::Server::default().start() {
        eprintln!("Start server failed: {}", e);
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    #[test]
    fn ext_name_test() {
        let p = Path::new("./config.ks");
        let ext_name = p.extension().map(|os_str| os_str.to_str());
        assert!(!matches!(ext_name, Some(Some("rs")) | None));
    }
}
