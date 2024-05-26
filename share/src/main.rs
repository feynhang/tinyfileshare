use std::{
    io::{BufWriter, ErrorKind, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr},
};

mod client;



pub const LOCALHOST: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.len() {
        0 => panic!("Invalid state, std::env::args() should not be empty!"),
        1 => {
            eprintln!("Missing file path argument");
            return;
        }
        _ => {
            while let Err(send_err) = send_file_paths(args[1..args.len()].iter()) {
                match send_err {
                    SendError::ConnectionError(err) => match err.kind() {
                        ErrorKind::ConnectionRefused | ErrorKind::TimedOut => {
                            if let Err(e) = start_server() {
                                eprintln!("Start service failed!\nDetail: {}", e);
                                return;
                            }
                        }
                        _ => {
                            eprintln!("Error occurred while connect to server!\nDetail: {}", err);
                            return;
                        }
                    },
                    SendError::StreamWriteError(e) => {
                        eprintln!(
                            "Error occurred while writing file paths to tcp stream!\n{}",
                            e
                        );
                        return;
                    }
                    SendError::ServerStartError => return,
                }
            }
            // let success = start_server();
            // if !success {
            //     eprintln!("Failed to start share_daemon service!");
            //     return;
            // }
        }
    }
}

fn read_port() -> u16 {
    todo!()
}

fn send_file_paths<T: AsRef<str>>(file_paths: impl Iterator<Item = T>) -> Result<(), SendError> {
    let port = read_port();
    let conn_res = std::net::TcpStream::connect(SocketAddr::new(LOCALHOST, port));
    if conn_res.is_err() {
        Err(SendError::ConnectionError(conn_res.unwrap_err()))
    } else {
        let mut writer = BufWriter::new(conn_res.unwrap());
        for file_path in file_paths {
            writer.write_fmt(format_args!("{}\n", file_path.as_ref())).unwrap();
        }
        if let Err(e) = writer.flush() {
            return Err(SendError::StreamWriteError(e));
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum SendError {
    ConnectionError(std::io::Error),
    StreamWriteError(std::io::Error),
    ServerStartError,
}

impl std::fmt::Display for SendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            SendError::ConnectionError(addr) => format!("Connect server failed!\nDetail: {}", addr),
            SendError::StreamWriteError(io_err) => {
                format!("Write data to stream failed\nDetail: {}", io_err)
            }
            SendError::ServerStartError => "Start server failed!".to_owned(),
        };
        write!(f, "{}", msg)
    }
}

impl std::error::Error for SendError {}

pub fn start_server() -> Result<(), ServiceError> {
    // different for win and other platform
    todo!()
}

#[cfg(target_os = "windows")]
pub fn start_service() -> Result<(), ServiceError> {
    todo!()
}

#[derive(Debug)]
#[cfg(target_os = "windows")]
pub enum ServiceError {
    AlreadyStarted,
    Todo,
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            ServiceError::AlreadyStarted => "todo!()",
            ServiceError::Todo => "todo!()",
        };

        write!(f, "{}", msg)
    }
}

impl std::error::Error for ServiceError {}

#[cfg(test)]
mod tests {
    use std::{
        net::{SocketAddr, TcpStream},
        // time::Duration,
    };

    #[test]
    fn test_conn_error() {
        let addr = SocketAddr::from(([127, 0, 0, 1], 20020));
        let res = TcpStream::connect(
            &addr, // Duration::from_secs(1),
        );
        if res.is_err() {
            eprintln!("connect addr: {}, error kind:\n{}", addr, unsafe {
                res.unwrap_err_unchecked().kind()
            });
        }
    }
}
