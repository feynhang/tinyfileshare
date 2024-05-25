use std::{
    fs::{File, OpenOptions},
    hash::{DefaultHasher, Hash, Hasher},
    io::{BufRead, BufReader, BufWriter, Read, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream},
    path::{Path, PathBuf},
    str::FromStr,
    sync::OnceLock,
    thread::JoinHandle,
};

use crossbeam::channel::Sender;
use serde::Serialize;
use toml::Table;

use crate::{filedata::FileData, log::LoggingPath, server::Server, CommonResult};

pub(crate) fn send_file_data(
    mut writer: BufWriter<TcpStream>,
    file_data: FileData,
) -> std::io::Result<()> {
    writer.write_fmt(format_args!("{}\n", file_data.name()))?;
    if let Some(data) = file_data.data() {
        writer.write_all(data)?;
    }
    writer.flush()?;
    Ok(())
}

pub(crate) fn read_file(path: &std::path::Path) -> std::io::Result<Option<FileData>> {
    let name = PathHandler::get_last_part(path);
    let mut data = vec![];
    let mut f = File::open(path)?;
    if f.read_to_end(&mut data)? == 0 {
        Ok(None)
    } else {
        data.shrink_to_fit();
        Ok(Some(FileData::new(name, data)))
    }
}

pub struct PathHandler;
impl PathHandler {
    #[cfg(windows)]
    pub fn get_home_path() -> std::path::PathBuf {
        std::path::PathBuf::from(std::env::var("USERPROFILE").unwrap())
    }

    #[cfg(not(windows))]
    pub fn get_home_path() -> std::path::PathBuf {
        std::path::PathBuf::from(std::env::var("HOME").unwrap())
    }

    pub fn get_exe_dir_path() -> PathBuf {
        let exe_path = std::env::current_exe().unwrap();
        let str_exe_path = exe_path.to_str().unwrap();
        std::path::PathBuf::from(
            &str_exe_path[0..str_exe_path.rfind(std::path::MAIN_SEPARATOR).unwrap()],
        )
    }

    pub fn get_last_part(path: &std::path::Path) -> String {
        let path_str = path.to_str().unwrap();
        path_str[path_str.rfind(std::path::MAIN_SEPARATOR).unwrap()..].to_owned()
    }


    pub(crate) fn get_default_log_dir() -> &'static Path {
        static mut DEFAULT_LOG_DIR: OnceLock<PathBuf> = OnceLock::new();
        unsafe {
            match DEFAULT_LOG_DIR.get() {
                Some(p) => p,
                None => {
                    let mut default_log_dir = PathHandler::get_exe_dir_path();
                    default_log_dir.push("log");
                    DEFAULT_LOG_DIR.set(default_log_dir).unwrap();
                    DEFAULT_LOG_DIR.get().unwrap()
                }
            }
        }
    }
    pub(crate) fn get_log_dir() -> PathBuf {
        // read log dir config from config file
        todo!()
    }
}

pub trait ReadAll {
    fn read_all(&mut self) -> std::io::Result<Vec<u8>>;
}

impl<W> ReadAll for W
where
    W: std::io::Read,
{
    fn read_all(&mut self) -> std::io::Result<Vec<u8>> {
        let mut ret = vec![];
        loop {
            let mut buf = [0_u8; 1024];
            let size = self.read(&mut buf)?;
            if size == 0 {
                break;
            }
            ret.extend(&buf[0..size]);
        }
        Ok(ret)
    }
}

#[derive(Debug)]
pub enum Handler {
    SendHandler {
        path: PathBuf,
        socket_addr: SocketAddr,
    },
    ReceiveHandler(TcpStream),
}

impl Handler {
    fn read_from_stream(stream: &mut TcpStream) -> std::io::Result<Data> {
        let mut first_line = String::new();
        let mut reader = BufReader::new(stream);
        if reader.read_line(&mut first_line)? == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Read first line failed! It should be a command: \"share\" or \"trans\"!!!",
            ));
        }
        first_line = first_line.trim_end().to_owned();
        match Command::from_str(&first_line)? {
            Command::Share => {
                let mut paths = vec![PathBuf::from(&first_line)];
                let mut line = String::new();
                while reader.read_line(&mut line)? > 0 {
                    paths.push(PathBuf::from(line.trim_end()));
                    line.clear();
                }
                Ok(Data::Paths(paths))
            }
            Command::Transfer => {
                let name = first_line.to_owned();
                let data = reader.read_all()?;
                Ok(Data::FileData(name, data))
            }
        }
    }

    pub(crate) fn handle(&mut self) -> Option<Data> {
        match self {
            Handler::SendHandler { path, socket_addr } => {
                let res = TcpStream::connect(socket_addr.clone());
                if let Err(e) = res {
                    path.log_err(e).unwrap();
                    return None;
                }
                let file_read_res = read_file(&path);
                if let Err(e) = file_read_res {
                    path.log_err(e).unwrap();
                    return None;
                }
                if let Some(file_data) = file_read_res.unwrap() {
                    if let Err(e) = send_file_data(BufWriter::new(res.unwrap()), file_data) {
                        path.log_err(e).unwrap();
                    }
                }
                None
            }
            Handler::ReceiveHandler(stream) => {
                if let Ok(data) = Self::read_from_stream(stream) {
                    match data {
                        Data::Paths(_) => {
                            //continue process in current thread
                            todo!()
                        }
                        Data::FileData(name, data) => {
                            //start a new worker thread to do
                            todo!()
                        }
                    }
                }
                None
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Data {
    Paths(Vec<PathBuf>),
    FileData(String, Vec<u8>),
}

pub struct ReadHandler;

impl ReadHandler {
    // each path should end with a line break when client send paths to server
}

#[derive(Debug, Clone, Copy)]
pub enum Command {
    Share,
    Transfer,
}

impl FromStr for Command {
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let command = s.to_uppercase();
        match command.as_str() {
            "SHARE" => Ok(Command::Share),
            "TRANS" => Ok(Command::Transfer),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Command is unsupported: {}", s),
            )),
        }
    }
}

// #[derive(Debug)]
// pub enum DataHandler {
//     Paths(TcpStream, Vec<PathBuf>),
//     FileData(FileData),
// }

// impl DataHandler {
//     pub(crate) fn handle(self) -> std::io::Result<()> {
//         match self {
//             DataHandler::Paths(stream, paths) => {
//                 let mut remote_addr = stream.peer_addr()?;
//                 let mut paths_itr = paths.iter();
//                 let path = paths_itr.next();
//                 if path.is_none() {
//                     return Ok(());
//                 }
//                 let file_data_res = read_file(path.unwrap())?;
//                 if file_data_res.is_none() {
//                     return Ok(());
//                 }
//                 send_file_data(BufWriter::new(stream), file_data_res.unwrap())?;
//                 for p in paths_itr.take(3) {
//                     let file_path = p.clone();
//                     remote_addr.set_port(remote_addr.port() + 1);
//                     std::thread::spawn(move || {
//                         let res = TcpStream::connect(remote_addr);
//                         if let Err(e) = res {
//                             file_path.log_err(e).unwrap();
//                             return;
//                         }
//                         let file_read_res = read_file(&file_path);
//                         if let Err(e) = file_read_res {
//                             file_path.log_err(e).unwrap();
//                             return;
//                         }
//                         if let Some(file_data) = file_read_res.unwrap() {
//                             if let Err(e) = send_file_data(BufWriter::new(res.unwrap()), file_data)
//                             {
//                                 file_path.log_err(e).unwrap();
//                             }
//                         }
//                     });
//                 }
//                 Ok(())
//             }
//             DataHandler::FileData(file_data) => {
//                 let mut save_path = Self::get_save_path();
//                 save_path.push(file_data.name());
//                 let mut writer = BufWriter::new(File::create(save_path)?);
//                 if let Some(data) = file_data.data() {
//                     writer.write_all(data)?;
//                 }
//                 writer.flush()?;
//                 Ok(())
//             }
//         }
//     }

//     fn get_save_path() -> PathBuf {
//         // read config file to get save path
//         todo!()
//     }
// }
