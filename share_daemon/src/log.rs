use std::io::{BufWriter, Stdout, Write};

use crate::global;

#[derive(Debug)]
pub struct Logger<W: Write>(W);
#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum LogLevel {
    Verbose,
    Debug,
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

impl<W> Logger<W>
where
    W: Write,
{
    pub(crate) fn log<T: std::fmt::Display + std::fmt::Debug>(&mut self, msg: T, level: LogLevel) {
        if level < global::log_level() {
            return;
        }
        let mut file_writer = BufWriter::new(&mut self.0);
        file_writer
            .write_fmt(format_args!(
                "[{}]-[{}]: {}",
                level,
                chrono::Local::now().timestamp(),
                msg
            ))
            .unwrap();
        file_writer.flush().unwrap();
    }
}

#[cfg(feature = "file_log")]
impl Logger<std::fs::File> {
    pub(crate) fn new<S: AsRef<str>>(extra_file_tag: Option<S>) -> Self {
        const FIXED_HEAD: &str = "tinyfileshare";
        const LOG_EXT: &str = ".log";
        let mut log_dir = crate::handler::PathHandler::get_log_dir();
        let file_name = if let Some(tag) = extra_file_tag {
            format!("{}-{}{}", FIXED_HEAD, tag.as_ref(), LOG_EXT)
        } else {
            FIXED_HEAD.to_owned() + LOG_EXT
        };

        log_dir.push(file_name);
        Self(
            std::fs::File::options()
                .append(true)
                .create(true)
                .open(log_dir)
                .unwrap(),
        )
    }
}

#[cfg(not(feature = "file_log"))]
impl Logger<Stdout> {
    pub(crate) fn new() -> Self {
        Self(std::io::stdout())
    }
}
