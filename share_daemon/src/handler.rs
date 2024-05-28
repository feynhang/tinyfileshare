use std::{
    fs::File,
    io::{BufRead, BufReader, BufWriter, Read, Write},
    net::{SocketAddr, TcpStream},
    path::{Path, PathBuf},
};

use crate::{dispatcher::Command, filedata::FileData};

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
        static mut DEFAULT_LOG_DIR: Option<PathBuf> = None;
        unsafe {
            DEFAULT_LOG_DIR.get_or_insert_with(|| {
                let mut default_log_dir = PathHandler::get_exe_dir_path();
                default_log_dir.push("log");
                default_log_dir
            })
        }
    }
}

// pub trait ReadAll {
//     fn read_all(&mut self) -> std::io::Result<Vec<u8>>;
// }

// impl<W> ReadAll for W
// where
//     W: std::io::Read,
// {
//     fn read_all(&mut self) -> std::io::Result<Vec<u8>> {
//         let mut ret = vec![];
//         loop {
//             let mut buf = [0_u8; 1024];
//             let size = self.read(&mut buf)?;
//             if size == 0 {
//                 break;
//             }
//             ret.extend(&buf[0..size]);
//         }
//         Ok(ret)
//     }
// }

#[derive(Debug)]
pub enum Handler {
    SendHandler {
        path: PathBuf,
        raw_server_addr: SocketAddr,
    },
    RecvHandler(TcpStream),
}

impl Handler {
    fn send_file_data(
        mut writer: BufWriter<TcpStream>,
        file_data: FileData,
    ) -> std::io::Result<()> {
        writer.write_fmt(format_args!(
            "{}\n{}\n",
            Command::Send,
            file_data.name()
        ))?;
        if let Some(data) = file_data.data() {
            writer.write_all(data)?;
        }
        writer.flush()?;
        Ok(())
    }

    fn read_file(path: &std::path::Path) -> std::io::Result<FileData> {
        let name = PathHandler::get_last_part(path);
        let mut data = vec![];
        let mut f = File::open(path)?;
        let size = f.read_to_end(&mut data)?;
        if size == 0 {
            return Ok(FileData::new(name, None));
        }
        data.shrink_to_fit();
        Ok(FileData::new(name, Some(data)))
    }

    pub(crate) fn handle(self) -> std::io::Result<()> {
        match self {
            Handler::SendHandler {
                path,
                raw_server_addr: socket_addr,
            } => {
                let stream = TcpStream::connect(socket_addr.clone())?;
                let file_data = Self::read_file(&path)?;
                Self::send_file_data(BufWriter::new(stream), file_data)?;
                return Ok(());
            }
            Handler::RecvHandler(stream) => {
                let mut buf_reader = BufReader::new(stream);
                let mut name = String::new();
                if let Ok(size) = buf_reader.read_line(&mut name) {
                    if size == 0 {}
                }
                Ok(())
            }
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
