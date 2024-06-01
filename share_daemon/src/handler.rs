use std::{
    fs::File,
    io::{BufRead, BufReader, BufWriter, Read, Write},
    net::{SocketAddr, TcpStream},
    path::{Path, PathBuf},
    str::FromStr,
};

use const_format::str_repeat;

use crate::{filedata::FileData, global, request::RequestCommand, response::Response};
const BUF_SIZE: usize = 4096;
const NEW_LINE: &str = "\r\n";

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
        let sep_index = if let Some(i) = path_str.rfind(std::path::MAIN_SEPARATOR) {
            i
        } else {
            path_str.rfind('/').unwrap()
        };
        path_str[sep_index..].to_owned()
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

pub trait ReadAll {
    fn read_all(&mut self) -> std::io::Result<Vec<u8>>;
    // fn read_fixed_vec(&mut self, vec: &mut Vec<u8>) -> std::io::Result<()>;
}

impl<R> ReadAll for R
where
    R: std::io::Read,
{
    fn read_all(&mut self) -> std::io::Result<Vec<u8>> {
        let mut ret = vec![];
        loop {
            let mut buf = [0; BUF_SIZE];
            let size = self.read(&mut buf)?;
            if size == 0 {
                break;
            }
            ret.extend(&buf[0..size]);
        }
        Ok(ret)
    }
}

pub trait WriteLine {
    fn write_line<T: std::fmt::Display>(&mut self, bytes: T) -> std::io::Result<()>;
}

impl<W> WriteLine for W
where
    W: std::io::Write,
{
    fn write_line<T: std::fmt::Display>(&mut self, bytes: T) -> std::io::Result<()> {
        self.write_fmt(format_args!("{}{}", bytes, NEW_LINE))
    }
}

#[allow(unused)]
#[derive(Debug)]
pub enum Handler {
    FileShareHandler(BufReader<TcpStream>),
    ReplyHandler(TcpStream, Response),
    FileRecvHandler(BufReader<TcpStream>),
    HostRegHandler(BufReader<TcpStream>),
    // ConfigureHandler(Config),
}

fn check_empty_line(line: &str, writer: &mut BufWriter<TcpStream>) -> std::io::Result<bool> {
    if !line.trim().is_empty() {
        writer.write_line(Response::InvalidRequest)?;
        return Ok(false);
    }
    Ok(true)
}

fn read_all_paths(reader: &mut BufReader<TcpStream>) -> std::io::Result<Vec<PathBuf>> {
    todo!()
}

impl Handler {
    fn read_file(path: &std::path::Path) -> std::io::Result<FileData> {
        let name = PathHandler::get_last_part(path);
        let mut data = vec![];
        let mut f = File::open(path)?;
        let size = f.read_to_end(&mut data)?;
        if size == 0 {
            return Ok(FileData::empty_file(name));
        }
        data.shrink_to_fit();
        Ok(FileData::new(name, data))
    }

    pub(crate) fn handle(self) -> std::io::Result<()> {
        match self {
            Handler::FileShareHandler(mut local_conn_reader) => {
                let mut line = String::new();
                local_conn_reader.read_line(&mut line)?;
                let mut local_conn_writer =
                    BufWriter::with_capacity(BUF_SIZE, local_conn_reader.get_ref().try_clone()?);
                if !check_empty_line(&line, &mut local_conn_writer)? {
                    return Ok(());
                }
                line.clear();
                local_conn_reader.read_line(&mut line)?;
                let ip_opt = global::registered_hosts().get(&line);
                if ip_opt.is_none() {
                    local_conn_writer.write_line(Response::UnregisteredHost)?;
                    return Ok(());
                }
                let ip = ip_opt.unwrap().clone();
                line.clear();
                local_conn_reader.read_line(&mut line)?;
                if !check_empty_line(&line, &mut local_conn_writer)? {
                    return Ok(());
                }
                line.clear();

                let mut file_paths = vec![];
                while local_conn_reader.read_line(&mut line)? != 0 {
                    if let Ok(p) = PathBuf::from_str(&line) {
                        if p.is_file() {
                            file_paths.push(p);
                        }
                    }
                }

                let addr = SocketAddr::new(ip, global::config().port());
                let conn_res = TcpStream::connect(addr);
                if let Err(e) = conn_res {
                    local_conn_writer.write_line(Response::ConnectHostFailed(line, ip, e))?;
                    return Ok(());
                }
                let mut target_writer = BufWriter::with_capacity(BUF_SIZE, conn_res.unwrap());
                for path in file_paths {
                    let file_data = Self::read_file(&path)?;
                    let file_size = file_data.data().len();
                    target_writer.write_line(RequestCommand::FileReceive.as_ref())?;
                    target_writer.write_line(format_args!("{}{}", NEW_LINE, file_data.name()))?;
                    target_writer.write_line(file_size)?;

                    local_conn_writer.write_line(format_args!("File:{}", file_data.name()))?;
                    let mut written_len = 0;
                    while written_len < file_size {
                        written_len += target_writer.write(file_data.data())?;
                        local_conn_writer.write_line(format_args!(
                            "{}{}",
                            Response::FileSendProgress(written_len as f64 / file_size as f64),
                            NEW_LINE,
                        ))?;
                    }

                    target_writer.write(str_repeat!(NEW_LINE, 2).as_bytes())?;
                    target_writer.flush()?;
                }
                return Ok(());
            }
            Handler::FileRecvHandler(mut buf_reader) => {
                let mut files_datas: Vec<FileData> = vec![];
                let mut line = String::new();
                loop {
                    buf_reader.read_line(&mut line)?;
                    if line.trim().is_empty() {
                        line.clear();
                        if buf_reader.read_line(&mut line)? > 0 {
                            let name = line.trim().to_owned();
                            line.clear();
                            if buf_reader.read_line(&mut line)? > 0 {
                                if let Ok(file_size) = usize::from_str_radix(&line, 10) {
                                    let mut data = vec![0_u8; file_size];
                                    buf_reader.read_exact(&mut data)?;
                                    files_datas.push(FileData::new(name, data));
                                    continue;
                                }
                            }
                            Self::ReplyHandler(buf_reader.into_inner(), Response::InvalidRequest);
                            return Ok(());
                        } else {
                            if files_datas.is_empty() {
                                Self::ReplyHandler(
                                    buf_reader.into_inner(),
                                    Response::InvalidRequest,
                                );
                                return Ok(());
                            }
                            break;
                        }
                    } else {
                        Self::ReplyHandler(buf_reader.into_inner(), Response::InvalidRequest);
                        return Ok(());
                    }
                }
                line.clear();
                let name_res = buf_reader.read_line(&mut line);
                if name_res.is_err() || name_res.unwrap() == 0 {
                    Self::ReplyHandler(buf_reader.into_inner(), Response::InvalidRequest);
                    return Ok(());
                }

                drop(buf_reader);
                let mut path = global::config().recv_dir().to_path_buf();
                for filedata in files_datas {
                    path.push(filedata.name());
                    let mut f = File::create(&path).unwrap();
                    f.write_all(filedata.data())?;
                }
                Ok(())
            }
            Handler::ReplyHandler(conn, msg) => {
                BufWriter::new(conn).write_fmt(format_args!("{}\n", msg))
            }
            Handler::HostRegHandler(reader) => {
                // let mut line = String::new();
                // global::registered_hosts().push(host);
                // Self::ReplyHandler(conn, Response::RegisterSuccess);
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs::File,
        io::{Read, Write},
        path::PathBuf,
    };

    use super::Handler;

    #[test]
    fn eof_test() {
        let mut path_buf = PathBuf::from("C:/Users/feyn/.cache/tinyfileshare/temp.txt");
        let f_data = Handler::read_file(&path_buf).unwrap();
        path_buf.pop();
        path_buf.push("temp_01.txt");
        let mut f_writer = std::fs::File::create(&path_buf).unwrap();
        f_writer.write_all(f_data.data()).unwrap();
        f_writer.write_all("\n\n".as_bytes()).unwrap();
        f_writer.flush().unwrap();
    }

    #[test]
    fn read_to_end_test() {
        let mut p = PathBuf::from("C:/Users/feyn/.cache/tinyfileshare/temp_01.txt");
        let mut data = vec![];
        let _ = File::open(&p).unwrap().read_to_end(&mut data).unwrap_or(0);
        p.pop();
        p.push("temp_02.txt");
        let mut w = std::fs::File::create(&p).unwrap();
    }
}
