use std::{fs::File, io::Write, sync::RwLock};

use chrono::NaiveDate;
use crossbeam_utils::atomic::AtomicCell;

use crate::global;

#[derive(Debug)]
pub struct Logger {
    level: LogLevel,
    kind: LoggerKind,
}
#[derive(Debug)]
pub enum LoggerKind {
    FileLogger,
    ConsoleLogger,
    NoLogger,
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

unsafe impl Send for LogLevel {}

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
    pub(crate) const fn no_logger() -> Self {
        Self {
            level: LogLevel::Off,
            kind: LoggerKind::NoLogger,
        }
    }
    pub(crate) const fn console_logger() -> Self {
        Self {
            level: LogLevel::Info,
            kind: LoggerKind::ConsoleLogger,
        }
    }

    pub(crate) const fn file_logger() -> Self {
        Self {
            level: LogLevel::Info,
            kind: LoggerKind::FileLogger,
        }
    }

    fn date() -> &'static AtomicCell<NaiveDate> {
        static TODAY: AtomicCell<NaiveDate> = AtomicCell::new(NaiveDate::MIN);
        if TODAY.load() == NaiveDate::MIN {
            TODAY.store(chrono::Local::now().date_naive());
        }
        &TODAY
    }

    fn update_date() -> bool {
        let new_day = chrono::Local::now().date_naive();
        unsafe {
            if Self::date().load() != new_day {
                Self::date().store(new_day);
                return true;
            }
            false
        }
    }

    fn open_log_file() -> File {
        let mut log_file_path = global::log_dir().to_owned();
        log_file_path.push(format!("{}.log", Self::date().load()));
        let mut open_options = File::options();
        if log_file_path.is_file() {
            open_options.append(true);
        } else {
            open_options.write(true).create(true);
        }
        match open_options.open(log_file_path) {
            Ok(f) => f,
            Err(e) => panic!(
                "Error occurred while open or create log file! Detail: {}",
                e
            ),
        }
    }

    pub(crate) fn log<T: std::fmt::Display + std::fmt::Debug + Send + Sync>(
        &self,
        msg: T,
        level: LogLevel,
    ) {
        if matches!(self.kind, LoggerKind::NoLogger) || level < self.level {
            return;
        }
        match &self.kind {
            LoggerKind::FileLogger => {
                static mut LOG_FILE: Option<RwLock<File>> = None;
                let mut f_guard = unsafe {
                    if Self::update_date() {
                        LOG_FILE = Some(RwLock::new(Self::open_log_file()));
                    }
                    match LOG_FILE.as_ref() {
                        Some(f_lock) => f_lock.write().unwrap(),
                        None => {
                            LOG_FILE = Some(RwLock::new(Self::open_log_file()));
                            LOG_FILE.as_ref().unwrap().write().unwrap()
                        }
                    }
                };
                f_guard
                    .write_fmt(format_args!(
                        "[{}]-[{}]: {}",
                        level,
                        chrono::Local::now().time(),
                        msg
                    ))
                    .expect("Write a log message to log file failed!");
            }
            LoggerKind::ConsoleLogger => {
                let mut stdout = std::io::stdout().lock();
                stdout
                    .write_fmt(format_args!(
                        "[{}]-[{}]: {}",
                        level,
                        chrono::Local::now().time(),
                        msg
                    ))
                    .expect("This should not happen: write an log to StdoutLock failed!");
                stdout.flush().unwrap();
            }
            LoggerKind::NoLogger => return,
        }
    }

    pub(crate) fn warn<T: std::fmt::Display + std::fmt::Debug + Send + Sync>(&self, msg: T) {
        self.log(msg, LogLevel::Warn)
    }

    pub(crate) fn info<T: std::fmt::Display + std::fmt::Debug + Send + Sync>(&self, msg: T) {
        self.log(msg, LogLevel::Info)
    }
    pub(crate) fn error<T: std::fmt::Display + std::fmt::Debug + Send + Sync>(&self, msg: T) {
        self.log(msg, LogLevel::Error)
    }
    pub(crate) fn debug<T: std::fmt::Display + std::fmt::Debug + Send + Sync>(&self, msg: T) {
        self.log(msg, LogLevel::Debug)
    }
    pub(crate) fn verbose<T: std::fmt::Display + std::fmt::Debug + Send + Sync>(
        &self,
        msg: T,
    ) {
        self.log(msg, LogLevel::Verbose)
    }
}
