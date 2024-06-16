use std::{
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    str::FromStr,
};

use interprocess::local_socket::{
    tokio::{SendHalf, Stream as LocalStream},
    traits::tokio::Stream,
};
use smol_str::format_smolstr;
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
    net::{tcp::OwnedWriteHalf, TcpStream},
};

pub trait WriteLine {
    async fn write_line<S: AsRef<str>>(&mut self, str: S) -> tokio::io::Result<()>;
}

impl<W> WriteLine for W
where
    W: AsyncWriteExt + ?Sized + std::marker::Unpin,
{
    async fn write_line<S: AsRef<str>>(&mut self, str: S) -> tokio::io::Result<()> {
        self.write_all(str.as_ref().as_bytes()).await?;
        self.write_all(consts::LINE_SEP.as_bytes()).await?;
        self.flush().await
    }
}

use crate::{consts, global, CommonResult};

pub(crate) async fn handle_local(stream: LocalStream) -> tokio::io::Result<()> {
    let (read_half, mut write_half) = stream.split();
    let mut reader = tokio::io::BufReader::new(read_half).take(2);
    let mut method_bytes = vec![];
    if reader.read_until(b' ', &mut method_bytes).await? != 0 && method_bytes.len() == 2 {
        method_bytes.pop();
        if let Ok(method_str) = std::str::from_utf8(&method_bytes) {
            match method_str {
                consts::request::SHARE => {
                    reader.set_limit(consts::MAX_HOSTNAME_LIMIT + 2);
                    let mut line = String::new();
                    if reader.read_line(&mut line).await? != 0 && !line.trim().is_empty() {
                        if let Some(ip) = global::config_store()
                            .await
                            .read()
                            .await
                            .inner()
                            .check_ip_by_name(line.trim())
                        {
                            let mut recv_paths = Vec::with_capacity(consts::PATHS_NUM_PER_REQUEST);
                            reader.set_limit(consts::FILE_PATH_LIMIT);
                            line.clear();
                            while recv_paths.len() < consts::PATHS_NUM_PER_REQUEST
                                && reader.read_line(&mut line).await? != 0
                                && !line.trim().is_empty()
                            {
                                let path = PathBuf::from(line.trim());
                                if !path.is_file() {
                                    write_half
                                        .write_line(consts::reply::THERES_INVALID_PATHS)
                                        .await?;
                                    return Ok(());
                                }
                                recv_paths.push(path);
                                reader.set_limit(consts::FILE_PATH_LIMIT);
                                line.clear();
                            }
                            if !recv_paths.is_empty() {
                                handle_file_send(*ip, write_half, recv_paths).await?;
                                return Ok(());
                            }
                        }
                    }
                }
                consts::request::HOST_REG => {
                    reader.set_limit(consts::MAX_HOSTNAME_LIMIT + consts::MAX_IP_LEN + 3);
                    let mut host_pair = String::new();
                    if reader.read_line(&mut host_pair).await? != 0 && !host_pair.trim().is_empty()
                    {
                        let pair: Vec<_> = host_pair.trim().split(':').collect();
                        if pair.len() == 2 {
                            if let Ok(ip) = IpAddr::from_str(pair[1]) {
                                if try_register_to_remote(ip).await.is_err() {
                                    write_half
                                        .write_line(consts::reply::REMOTE_REGISTRATION_FAILED)
                                        .await?;
                                    return Ok(());
                                }
                                if let Ok(option_ip) = try_register_to_local(pair[0], ip).await {
                                    if let Some(replaced) = option_ip {
                                        write_half
                                            .write_line(format_smolstr!(
                                                "{} {}",
                                                consts::reply::REPLACED_IP,
                                                replaced
                                            ))
                                            .await?;
                                    } else {
                                        write_half
                                            .write_line(consts::reply::REGISTRATION_SUCCEED)
                                            .await?;
                                    }
                                } else {
                                    write_half
                                        .write_line(consts::reply::LOCAL_REGISTRATION_FAILED)
                                        .await?;
                                }
                                return Ok(());
                            }
                        }
                    }
                }
                _ => (),
            }
        }
    }
    write_half
        .write_line(consts::reply::INVALID_REQUEST)
        .await?;
    Ok(())
}

async fn try_register_to_remote(remote_ip: IpAddr) -> CommonResult<()> {
    let remote_endpoint = SocketAddr::from((
        remote_ip,
        global::config_store()
            .await
            .read()
            .await
            .inner()
            .listener_addr
            .port(),
    ));
    let remote_stream = TcpStream::connect(remote_endpoint).await?;
    let (read_half, mut write_half) = remote_stream.into_split();
    if read_half.readable().await.is_ok() {
        let mut reader = BufReader::new(read_half).take(50);
        let mut line = String::new();
        if reader.read_line(&mut line).await? != 0 {
            match line.trim() {
                consts::reply::WAITING => {
                    return Ok(());
                }
                consts::reply::UNREGISTERED_REMOTE => {
                    write_half.write_line(consts::request::REG_ME).await?;
                    reader.set_limit(50);
                    line.clear();
                    if reader.read_line(&mut line).await? != 0
                        && line.trim() == consts::reply::REGISTRATION_SUCCEED
                    {
                        return Ok(());
                    }
                }
                _ => (),
            }
        }
    }

    Err(crate::error::CommonError::Failed)
}

async fn try_register_to_local(hostname: &str, ip: IpAddr) -> CommonResult<Option<IpAddr>> {
    let conf_store_lock = global::config_store().await;
    let mut config_store = conf_store_lock.write().await;
    let replaced = config_store.mut_inner().register_host(hostname, ip);
    config_store.save_to_file().await?;
    Ok(replaced)
}

async fn reply_unexpected(
    local_write_half: &mut SendHalf,
    mut write_half: OwnedWriteHalf,
) -> tokio::io::Result<()> {
    write_half
        .write_line(consts::reply::UNEXPECTED_RESPONSE)
        .await?;
    write_half.flush().await?;
    write_half.forget();
    local_write_half
        .write_line(consts::reply::UNEXPECTED_REMOTE_RESPONSE)
        .await?;
    local_write_half.flush().await?;
    Ok(())
}

async fn transfer_files(
    mut local_write_half: SendHalf,
    addr: SocketAddr,
    files_paths: Vec<PathBuf>,
) -> tokio::io::Result<()> {
    let conn = TcpStream::connect(addr).await?;
    let mut writer = BufWriter::new(conn);
    writer
        .write_line(constcat::concat!(
            consts::request::TRANSFER_START,
            consts::LINE_SEP
        ))
        .await?;
    for p in files_paths {
        let name = p
            .file_name()
            .expect("Get file name failed, this should not happen!")
            .to_string_lossy();
        let mut f = File::open(&p).await?;
        let file_size = f.metadata().await.unwrap().len();
        writer
            .write_line(format_smolstr!("{}:{}", name, file_size))
            .await?;
        let mut size_count = 0;
        loop {
            let mut buf = [0_u8; consts::FILE_TRANS_BUF_SIZE];
            let read_size = f.read(&mut buf).await?;
            if read_size == 0 {
                break;
            }
            size_count += read_size;
            writer.write_all(&buf[0..read_size]).await?;
            writer.flush().await?;
            local_write_half
                .write_line(format_smolstr!(
                    "{} {}:{}",
                    consts::reply::PROGRESS,
                    name,
                    size_count as f64 / file_size as f64
                ))
                .await?;
        }
        writer.write_all(consts::LINE_SEP.as_bytes()).await?;
    }
    local_write_half
        .write_line(consts::reply::TRANSFER_END)
        .await?;
    writer.write_line(consts::reply::TRANSFER_END).await?;
    Ok(())
}

async fn handle_file_send(
    ip: IpAddr,
    mut local_write_half: SendHalf,
    files_paths: Vec<PathBuf>,
) -> tokio::io::Result<()> {
    let conf_store_lock = global::config_store().await;
    let config_store = conf_store_lock.read().await;
    if config_store.inner().check_ip_registered(ip) {
        let listener_port = config_store.inner().listener_addr.port();
        let remote_addr = SocketAddr::from((ip, listener_port));
        if let Ok(remote_stream) = TcpStream::connect(remote_addr).await {
            let (remote_read_half, mut remote_write_half) = remote_stream.into_split();
            let mut remote_reader = BufReader::with_capacity(128, remote_read_half).take(25);
            let mut line = String::with_capacity(20);
            if remote_reader.read_line(&mut line).await? != 0 {
                match line.trim() {
                    consts::reply::WAITING => {
                        let prepared_port = listener_port + 1;
                        remote_write_half
                            .write_line(format_smolstr!(
                                "{} {}",
                                consts::request::PORT_PREPARE,
                                prepared_port
                            ))
                            .await?;
                        remote_reader.set_limit(20);
                        line.clear();
                        if remote_reader.read_line(&mut line).await? != 0 {
                            let resp_parts: Vec<&str> = line.trim().split(' ').collect();
                            let mut is_valid_req = true;
                            let mut transmission_port = 0;
                            match resp_parts.len() {
                                1 if resp_parts[0] == consts::reply::PORT_OK => {
                                    transmission_port = prepared_port;
                                }
                                2 if resp_parts[0] == consts::reply::PORT_CONFIRM => {
                                    if let Ok(port) = u16::from_str_radix(resp_parts[1], 10) {
                                        transmission_port = port;
                                    } else {
                                        reply_unexpected(&mut local_write_half, remote_write_half)
                                            .await?;
                                        return Ok(());
                                    }
                                }
                                _ => is_valid_req = false,
                            }
                            if is_valid_req {
                                return transfer_files(
                                    local_write_half,
                                    SocketAddr::from((ip, transmission_port)),
                                    files_paths,
                                )
                                .await;
                            }
                        }
                    }
                    consts::reply::UNREGISTERED_REMOTE => {
                        local_write_half
                            .write_line(consts::reply::UNREGISTERED_REMOTE)
                            .await?;
                        local_write_half.flush().await?;
                        return Ok(());
                    }
                    _ => (),
                }
            }
            reply_unexpected(&mut local_write_half, remote_write_half).await?;
        } else {
            local_write_half
                .write_line(format_smolstr!(
                    "{} {}",
                    consts::reply::UNREACHABLE_SOCKETADDRESS,
                    remote_addr
                ))
                .await?;
        }
    } else {
        local_write_half
            .write_line(consts::reply::UNREGISTERED_LOCAL)
            .await?;
    }
    local_write_half.flush().await?;
    Ok(())
}

pub(crate) async fn handle_remote(stream: TcpStream, peer_addr: SocketAddr) -> CommonResult<()> {
    let (read_half, mut write_half) = stream.into_split();
    if global::config_store()
        .await
        .read()
        .await
        .inner()
        .check_ip_registered(peer_addr.ip())
    {
        write_half.write_line(consts::reply::WAITING).await?;
        if read_half.readable().await.is_ok() {}
    } else {
        write_half
            .write_line(consts::reply::UNREGISTERED_REMOTE)
            .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn u8_print_test() {
        let b = b'S';
        assert_eq!(format!("{}", char::from(b)).as_bytes(), &[b]);
        println!(
            "str of b: {}, char of b: {}, direct b: {}",
            std::str::from_utf8(&[b]).unwrap(),
            char::from(b),
            b
        );
    }
}
