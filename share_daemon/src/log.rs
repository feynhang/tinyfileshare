use std::{
    fs::File,
    io::Write,
    path::PathBuf,
    sync::RwLock,
};


use crate::{global, server};

#[derive(Debug)]
pub struct Logger {
    level: LogLevel,
    kind: LoggerKind,
}
#[derive(Debug)]
pub enum LoggerKind {
    FileLogger,
    ConsoleLogger,
}

#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub enum LogLevel {
    Verbose,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
    Off,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let level_str = match self {
            LogLevel::Verbose => "VERBOSE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Off => "",
        };
        write!(f, "{}", level_str)
    }
}

#[allow(unused)]
impl Logger {
    pub(crate) fn console_logger() -> Self {
        Self {
            level: Default::default(),
            kind: LoggerKind::ConsoleLogger,
        }
    }

    pub(crate) fn file_logger() -> Self {
        Self {
            level: Default::default(),
            kind: LoggerKind::FileLogger,
        }
    }

    fn get_log_file_path() -> PathBuf {
        let mut log_file_path = server::config_store().log_dir().to_path_buf();
        log_file_path.push(format!("{}.log", chrono::Local::now().date_naive()));
        log_file_path
    }

    pub(crate) fn log<T: std::fmt::Display + std::fmt::Debug>(&mut self, msg: T, level: LogLevel) {
        if level < self.level {
            return;
        }
        match &mut self.kind {
            LoggerKind::FileLogger => {
                static mut LOG_FILE: Option<RwLock<File>> = None;
                let log_file_path = Self::get_log_file_path();
                let log_file = unsafe {
                    if log_file_path.is_file() {
                        LOG_FILE.get_or_insert(RwLock::new(
                            File::options()
                                .append(true)
                                .open(log_file_path)
                                .expect("Unexpected: open log file failed!"),
                        ))
                    } else {
                        LOG_FILE = Some(RwLock::new(
                            File::options()
                                .write(true)
                                .create(true)
                                .open(log_file_path)
                                .expect("Unexpected: create log file failed!"),
                        ));
                        LOG_FILE.as_mut().unwrap()
                    }
                };
                let mut writer = log_file.write().expect("Get log file write lock failed!");
                writer
                    .write_fmt(format_args!(
                        "[{}]-[{}]: {}",
                        level,
                        chrono::Local::now().time(),
                        msg
                    ))
                    .unwrap();
                writer.flush().unwrap();
            }
            LoggerKind::ConsoleLogger => {
                let mut stdout = std::io::stdout();
                stdout
                    .write_fmt(format_args!(
                        "[{}]-[{}]: {}",
                        level,
                        chrono::Local::now().time(),
                        msg
                    ))
                    .expect("This should not happen: write an log to sync stdout failed!");
                stdout.flush().unwrap();
            }
        }
    }

    pub(crate) fn warn<T: std::fmt::Display + std::fmt::Debug>(&mut self, msg: T) {
        self.log(msg, LogLevel::Warn)
    }

    pub(crate) fn info<T: std::fmt::Display + std::fmt::Debug>(&mut self, msg: T) {
        self.log(msg, LogLevel::Info)
    }
    pub(crate) fn error<T: std::fmt::Display + std::fmt::Debug>(&mut self, msg: T) {
        self.log(msg, LogLevel::Error)
    }
    pub(crate) fn debug<T: std::fmt::Display + std::fmt::Debug>(&mut self, msg: T) {
        self.log(msg, LogLevel::Debug)
    }
    pub(crate) fn verbose<T: std::fmt::Display + std::fmt::Debug>(&mut self, msg: T) {
        self.log(msg, LogLevel::Verbose)
    }
}
