use std::{
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    str::FromStr,
};

use interprocess::local_socket::{tokio::Stream as LocalStream, traits::tokio::Stream};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};

use crate::{
    config,
    consts::{self, request},
    server, CommonResult,
};

pub const PATHS_NUM_PER_REQUEST: usize = config::DEFAULT_NUM_WORKERS as usize - 1;
pub const FILE_PATH_LIMIT: u64 = 500;
pub const MAX_HOSTNAME_LIMIT: u64 = 16;

pub(crate) async fn handle_local(stream: LocalStream) -> tokio::io::Result<()> {
    let (read_half, mut write_half) = stream.split();
    let mut reader = tokio::io::BufReader::new(read_half).take(2);
    let mut method_bytes = vec![];
    if reader.read_until(b' ', &mut method_bytes).await? != 0 && method_bytes.len() == 2 {
        method_bytes.pop();
        if method_bytes[0] == request::SHARE {
            reader.set_limit(MAX_HOSTNAME_LIMIT+2);
            let mut line = String::new();
            if reader.read_line(&mut line).await? != 0 && !line.trim().is_empty() {
                if let Some(ip) = server::config_store().get_ip_by_name(line.trim()) {
                    let mut recv_paths = vec![];
                    reader.set_limit(FILE_PATH_LIMIT);
                    line.clear();
                    let mut i = 0;
                    while i < PATHS_NUM_PER_REQUEST
                        && reader.read_line(&mut line).await? != 0
                        && !line.trim().is_empty()
                    {
                        let path = PathBuf::from(line.trim());
                        if !path.is_file() {
                            write_half.write_all(b"THERES_INVALID_PATH").await?;
                            return Ok(());
                        }
                        recv_paths.push(path);
                        line.clear();
                        i += 1;
                    }
                    if !recv_paths.is_empty() {
                        handle_file_send(*ip, write_half, recv_paths).await?;
                        return Ok(());
                    }
                }
            }
        }
        if method_bytes[0] == request::HOST_REG {
            const MAX_IP_LEN: u64 = 46;
            reader.set_limit(MAX_HOSTNAME_LIMIT + MAX_IP_LEN + 3);
            let mut host_pair = String::new();
            if reader.read_line(&mut host_pair).await? != 0 && !host_pair.trim().is_empty() {
                let pair: Vec<_> = host_pair.split(':').collect();
                if pair.len() == 2 {
                    if let Ok(ip) = IpAddr::from_str(pair[1]) {
                        if let Some(replaced) = server::config_store().register_host(pair[0], ip) {
                            write_half
                                .write_all(
                                    format!("{} {}", consts::reply::REPLACED_IP, replaced)
                                        .as_bytes(),
                                )
                                .await?;
                        } else {
                            write_half
                                .write_all(consts::reply::REGISTERED_SUCCESS.as_bytes())
                                .await?;
                        }
                        write_half.flush().await?;
                        return Ok(());
                    }
                }
            }
        }
    }
    write_half
        .write_all(consts::reply::INVALID_REQUEST.as_bytes())
        .await?;
    write_half.flush().await?;
    Ok(())
}

async fn handle_file_send(
    ip: IpAddr,
    mut write_half: interprocess::local_socket::tokio::SendHalf,
    recv_paths: Vec<PathBuf>,
) -> tokio::io::Result<()> {
    if server::config_store().check_ip_registered(ip) {
        let remote_addr = SocketAddr::from((ip,server::config_store().start_port()));
        let stream_res = TcpStream::connect(remote_addr).await;
        if stream_res.is_err() {
            write_half.write_all(consts::reply::UNREACHABLE_HOST.as_bytes()).await?;
            write_half.flush().await?;
            return Ok(());
        }
        let mut remote_stream = stream_res.unwrap();
        let (remote_read_half, mut remote_write_half) = remote_stream.split();
        remote_write_half.write_all(&[consts::request::TEST_REACHABLE]).await?;
        let mut reader = BufReader::new(remote_read_half).take(2);
        let mut buf = vec![];
        if reader.read_to_end(&mut buf).await? > 0  {
            todo!();
            // return Ok(());
        } else {
            remote_write_half.write_all(consts::reply::UNEXPECTED_RESPONSE.as_bytes()).await?;
            remote_write_half.flush().await?;
            return Ok(());
        }
    }
    write_half.write_all(consts::reply::UNREGISTERED_HOST.as_bytes()).await?;
    write_half.flush().await?;
    Ok(())
}

pub(crate) async fn handle_remote(stream: TcpStream, peer_addr: SocketAddr) -> CommonResult<()> {
    todo!()
}
