use std::{
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    str::FromStr,
};

use interprocess::local_socket::{tokio::Stream as LocalStream, traits::tokio::Stream};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::{
    config,
    consts::{self, request_head},
    response::FailureResponse,
    server, CommonResult,
};


const MAX_PATHS_PER_TIME: usize = config::DEFAULT_NUM_WORKERS as usize - 1;
const FILE_PATH_LIMIT: usize = 500;

pub(crate) async fn handle_local(stream: LocalStream) -> CommonResult<()> {
    let (read_half, mut write_half) = stream.split();
    let mut reader = tokio::io::BufReader::new(read_half).take(100);
    let mut line_bytes = vec![];
    if reader.read_until(b'\n', &mut line_bytes).await? != 0 {
        let mut parts = line_bytes.split(|b| *b == b' ');
        if parts.clone().count() == 2 {
            if let Ok(args_str) = std::str::from_utf8(parts.nth(1).unwrap()) {
                let pair: Vec<&str> = args_str.split(':').collect();
                match parts.nth(0).unwrap() {
                    [request_head::HOST_REG] if pair.len() == 2 => {
                        if pair[0].len() <= 16 {
                            let name = pair[0];
                            if let Ok(addr) = IpAddr::from_str(pair[1]) {
                                if let Some(ip) = server::config_store().register_host(name, addr) {
                                    write_half
                                        .write_all(
                                            format!(
                                                "{} {}",
                                                consts::response_head::REPLACED_HOST,
                                                ip
                                            )
                                            .as_bytes(),
                                        )
                                        .await?;
                                } else {
                                    write_half
                                        .write_all(
                                            consts::response_head::REGISTERED_SUCCESS.as_bytes(),
                                        )
                                        .await?;
                                }

                                write_half.flush().await?;
                                return Ok(());
                            }
                        }
                    }
                    [request_head::SHARE] => {
                        if let Ok(ip) = IpAddr::from_str(args_str) {
                            if server::config_store().check_ip_registered(ip) {
                                let mut invalid_paths = vec![];
                                let mut files_paths = Vec::with_capacity(MAX_PATHS_PER_TIME);

                                let mut line = String::with_capacity(FILE_PATH_LIMIT);
                                for _ in 0..MAX_PATHS_PER_TIME {
                                    reader.set_limit(FILE_PATH_LIMIT as u64);
                                    if reader.read_line(&mut line).await? != 0 && !line.is_empty() {
                                        let path = PathBuf::from(&line);
                                        if !path.is_file() {
                                            invalid_paths.push(line.clone());
                                            continue;
                                        }
                                        files_paths.push(path);
                                    }
                                    line.clear();
                                }
                                if !files_paths.is_empty() {
                                    if !invalid_paths.is_empty() {
                                        write_half
                                            .write_all(
                                                format!(
                                                    "{}\r\n{}",
                                                    consts::response_head::INVALID_PATHS,
                                                    invalid_paths.as_slice().join("\r\n")
                                                )
                                                .as_bytes(),
                                            )
                                            .await?;
                                    } else {
                                        write_half
                                            .write_all(
                                                consts::response_head::ALL_PATHS_RECEIVED
                                                    .as_bytes(),
                                            )
                                            .await?;
                                    }
                                } else {
                                    write_half
                                        .write_all(
                                            consts::response_head::ALL_PATHS_INVALID.as_bytes(),
                                        )
                                        .await?;
                                }
                                write_half.flush().await?;
                                // start files share to destination remote host task
                                return Ok(());
                            }
                        }
                    }
                    _ => (),
                }
            }
        }
    }
    write_half
        .write_all(FailureResponse::InvalidRequest.to_string().as_bytes())
        .await?;
    write_half.flush().await?;
    Ok(())
}

pub(crate) async fn handle_remote(stream: TcpStream, peer_addr: SocketAddr) -> CommonResult<()> {
    todo!()
}