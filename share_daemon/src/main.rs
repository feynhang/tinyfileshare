#![allow(unused)]
use std::{
    net::{IpAddr, SocketAddr},
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

use faccess::{AccessMode, PathExt};
use fshare_server::server;
use interprocess::local_socket::{GenericNamespaced, ToNsName};
use smol_str::{SmolStr, StrExt};

#[inline(always)]
fn check_symlink(p: &Path) -> anyhow::Result<()> {
    if p.is_symlink() {
        return Err(anyhow::anyhow!("Symbolic link is not supported!"));
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct DirPath(PathBuf);

impl From<DirPath> for PathBuf {
    fn from(value: DirPath) -> Self {
        value.0
    }
}

impl std::fmt::Display for DirPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.to_string_lossy())
    }
}

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
            && path.access(AccessMode::WRITE | AccessMode::READ).is_ok()
        {
            return Ok(Self(path.to_path_buf()));
        }
        Err(anyhow::anyhow!("Invalid configuration file path"))
    }
}

#[derive(Debug, Clone)]
struct SockName(SmolStr);

impl From<SockName> for SmolStr {
    fn from(value: SockName) -> Self {
        value.0
    }
}

impl std::str::FromStr for SockName {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(anyhow::anyhow!("socket name cannot be empty!"));
        }
        let name_res = s.to_ns_name::<GenericNamespaced>();
        if name_res.is_ok()
            && interprocess::local_socket::ListenerOptions::new()
                .name(name_res.unwrap())
                .create_tokio()
                .is_ok()
        {
            return Ok(Self(s.into()));
        }
        Err(anyhow::anyhow!("Invalid IPC socket name!"))
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

const DEFAULT_LOG_FILE_NAME: &str = "tinyshare.log";

mod arg_id {
    pub const LOG_DIR: &str = "log_dir";
    pub const LOG_LEVEL: &str = "log_level";

    pub const CONFIG: &str = "config";

    pub const ADDR: &str = "addr";
    pub const IP: &str = "ip";
    pub const PORT: &str = "port";

    pub const IPC_SOCKET_NAME: &str = "ipc_socket";

    pub const FILES_SAVE_DIR: &str = "save_dir";
    pub const PORT: &str = "port";
    pub const SVR_SOCK_NAME: &str = "server_socket";
    pub const CLT_SOCK_NAME: &str = "client_socket";
    pub const LOG_LEVEL: &str = "log_level";
    pub const DEFAULT_SAVE_DIR: &str = "save_dir";

    pub const SAVE_DIR: &str = "save_dir";
}

fn main() {
    let mut matches = clap::Command::new(env!("CARGO_CRATE_NAME"))
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
            clap::Arg::new(arg_id::IP)
                .long(arg_id::IP)
                .value_parser(clap::value_parser!(IpAddr)),
        )
        .arg(
            clap::Arg::new(arg_id::PORT)
                .short('p')
                .long(arg_id::PORT)
                .value_parser(clap::value_parser!(u16).range(3000..)),
        )
        .arg(
            clap::Arg::new(arg_id::ADDR)
                .short('a')
                .long(arg_id::ADDR)
                .value_parser(clap::value_parser!(SocketAddr))
                .conflicts_with_all([arg_id::IP, arg_id::PORT]),
        )
        .arg(
            clap::Arg::new(arg_id::IPC_SOCKET_NAME)
                .long(arg_id::IPC_SOCKET_NAME)
                .value_parser(clap::value_parser!(SockName)),
        )
        .arg(
            clap::Arg::new(arg_id::LOG_LEVEL)
                .long(arg_id::LOG_LEVEL)
                .value_parser(clap::builder::EnumValueParser::<LogLevel>::new())
                .ignore_case(true),
        )
        .arg(

            clap::Arg::new(arg_id::FILES_SAVE_DIR)
                .long(arg_id::FILES_SAVE_DIR))
        .arg(
            clap::Arg::new(arg_id::DEFAULT_SAVE_DIR)
                .long(arg_id::DEFAULT_SAVE_DIR))
                .arg(
            clap::Arg::new(arg_id::SAVE_DIR)
                .long(arg_id::SAVE_DIR)
                .value_parser(clap::value_parser!(DirPath)),
        )
        .get_matches();
    let mut server = fshare_server::server::Server::default();
    if let Some(mut log_dir) = matches.remove_one::<DirPath>(arg_id::LOG_DIR) {
        log_dir.push(DEFAULT_LOG_FILE_NAME);
        let create_log_res = std::fs::File::create(log_dir.as_path());
        if let Err(e) = create_log_res {
            eprintln!("Failed to open or create a log file from the specified directory: \"{}\". Detail: {}", log_dir,e);
            return;
        }
        server.set_log_target(env_logger::Target::Pipe(Box::new(create_log_res.unwrap())));
    }
    if let Some(log_level) = matches.remove_one::<LogLevel>(arg_id::LOG_LEVEL) {
        server.set_max_log_level(log_level.to_filter());
    }

    if let Some(config_path) = matches.remove_one::<ConfigPath>(arg_id::CONFIG) {
        if let Err(e) = server.load_config_file(&config_path.0) {
            log::error!(
                "Load from specified config file failed! Config path: \"{}\", error detail: {}",
                config_path,
                e
            );
        }
    }

    if let Some(addr) = matches.remove_one::<SocketAddr>(arg_id::ADDR) {
        server.set_listener_addr(addr);
    } else {
        if let Some(ip) = matches.remove_one::<IpAddr>(arg_id::IP) {
            server.set_listener_ip(ip);
        }
        if let Some(port) = matches.remove_one::<u16>(arg_id::PORT) {
            server.set_listener_port(port);
        }
    }
    if let Some(socket_name) = matches.remove_one::<SockName>(arg_id::IPC_SOCKET_NAME) {
        server.set_ipc_socket_name(socket_name.into());
    }

    if let Some(files_save_dir) = matches.remove_one::<DirPath>(arg_id::FILES_SAVE_DIR) {
        server.set_save_dir(files_save_dir);
    }

    if let Err(e) = server::Server::default().start() {
        eprintln!("Start server failed: \ndetail: {}", e);
    }
}

#[cfg(test)]
mod tests {}
