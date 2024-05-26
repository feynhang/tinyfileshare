use std::{
    fs::File,
    hash::{DefaultHasher, Hash, Hasher},
    io::{BufWriter, Stdout, Write},
    path::{Path, PathBuf},
    sync::OnceLock,
};

use crate::handler::PathHandler;

#[cfg(feature = "file_log")]
pub(crate) fn global_logger() -> &'static mut Logger<File> {
    static mut GLOBAL_LOGGER: OnceLock<Logger<File>> = OnceLock::new();
    unsafe {
        match GLOBAL_LOGGER.get_mut() {
            Some(l) => l,
            None => {
                GLOBAL_LOGGER.set(Logger::new(LogLevel::Warn, None)).unwrap();
                GLOBAL_LOGGER.get_mut().unwrap()
            }
        }
    }
}

#[cfg(not(feature = "file_log"))]
pub(crate) fn global_logger() -> &'static mut Logger<Stdout> {
    static mut GLOBAL_LOGGER: OnceLock<Logger<Stdout>> = OnceLock::new();
    unsafe {
        match GLOBAL_LOGGER.get_mut() {
            Some(l) => l,
            None => {
                GLOBAL_LOGGER.set(Logger::new(LogLevel::Warn)).unwrap();
                GLOBAL_LOGGER.get_mut().unwrap()
            }
        }
    }
}

#[derive(Debug)]
pub struct Logger<W: Write> {
    target: W,
    level: LogLevel,
}

#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
    Off,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let level_str = match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Fatal => "FATAL",
            LogLevel::Off => "",
        };
        write!(f, "{}", level_str)
    }
}

impl<W> Logger<W>
where
    W: Write,
{
    pub(crate) fn set_level(&mut self, new_level: LogLevel) {
        self.level = new_level
    }

    pub(crate) fn level(&self) -> LogLevel {
        self.level
    }

    pub(crate) fn log<T: std::fmt::Display + std::fmt::Debug>(&mut self, msg: T) {
        let mut file_writer = BufWriter::new(&mut self.target);
        file_writer
            .write_fmt(format_args!(
                "[{}]-[{}]: {}",
                self.level,
                chrono::Local::now().timestamp(),
                msg
            ))
            .unwrap();
        file_writer.flush().unwrap();
    }
}

#[cfg(feature = "file_log")]
impl Logger<File> {
    pub(crate) fn new<S: AsRef<str>>(level: LogLevel, extra_file_tag: Option<S>) -> Self {
        const FIXED_HEAD: &str = "tinyfileshare";
        const LOG_EXT: &str = ".log";
        let mut log_dir = PathHandler::get_log_dir();
        let file_name = if let Some(tag) = extra_file_tag {
            format!("{}-{}{}", FIXED_HEAD, tag.as_ref(), LOG_EXT)
        } else {
            FIXED_HEAD.to_owned() + LOG_EXT
        };

        log_dir.push(file_name);
        Self {
            target: File::options()
                .append(true)
                .create(true)
                .open(log_dir)
                .unwrap(),
            level,
        }
    }
}

#[cfg(not(feature = "file_log"))]
impl Logger<Stdout> {
    pub(crate) fn new(level: LogLevel) -> Self {
        Self {
            target: std::io::stdout(),
            level,
        }
    }
}

fn get_log_path(file_path: &std::path::Path) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    file_path.hash(&mut hasher);

    let mut log_path = PathHandler::get_log_dir();
    log_path.push(format!(
        "{}-failed-{}.log",
        PathHandler::get_last_part(file_path),
        hasher.finish()
    ));
    log_path
}