use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
    str::FromStr,
};

use interprocess::local_socket::{
    tokio::{SendHalf, Stream as LocalStream},
    traits::tokio::Stream,
    GenericNamespaced, ToNsName,
};
use smol_str::{format_smolstr, ToSmolStr};
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
    net::{tcp::OwnedWriteHalf, TcpListener, TcpStream},
    sync::RwLock,
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

use crate::{consts, error::CommonError, global, CommonResult};

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
                        if let Some(host) = global::config_store()
                            .await
                            .read()
                            .await
                            .get_host(line.trim())
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
                                        .write_line(consts::reply::ANY_PATH_INVALID)
                                        .await?;
                                    return Ok(());
                                }
                                recv_paths.push(path);
                                reader.set_limit(consts::FILE_PATH_LIMIT);
                                line.clear();
                            }
                            if !recv_paths.is_empty() {
                                handle_file_send(*host, write_half, recv_paths).await?;
                                return Ok(());
                            }
                        } else {
                            write_half
                                .write_line(consts::reply::UNREGISTERED_LOCAL)
                                .await?;
                            return Ok(());
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
                            if let Ok(addr) = SocketAddr::from_str(pair[1]) {
                                if let Err(CommonError::FailureResponse(fail_resp)) =
                                    try_register_to_remote(addr).await
                                {
                                    write_half.write_line(fail_resp).await?;
                                    return Ok(());
                                }
                                if let Ok(option_ip) = try_register_to_local(pair[0], addr).await {
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
                                            .write_line(consts::reply::REGISTRATION_SUCCEEDED)
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

fn checked_send_port(port: u16) -> u16 {
    if port.wrapping_add(1) == 0 {
        port.wrapping_add(2)
    } else {
        port + 1
    }
}

async fn handle_file_send(
    remote_addr: SocketAddr,
    mut local_write_half: SendHalf,
    files_paths: Vec<PathBuf>,
) -> tokio::io::Result<()> {
    if let Ok(remote_stream) = TcpStream::connect(remote_addr).await {
        let (remote_read_half, mut remote_write_half) = remote_stream.into_split();
        remote_write_half
            .write_line(format_smolstr!(
                "{} {}",
                consts::request::PORT_EXPECTED,
                checked_send_port(remote_addr.port())
            ))
            .await?;
        let mut line = String::with_capacity(50);
        let mut remote_reader = BufReader::with_capacity(128, remote_read_half).take(50);
        if remote_reader.read_line(&mut line).await? != 0 && !line.trim().is_empty() {
            let resp_parts: Vec<&str> = line.trim().split(' ').collect();
            if resp_parts[0] == consts::reply::RECV_REFUSED {
                local_write_half
                    .write_line(consts::reply::TRANS_REMOTE_REFUSED)
                    .await?;
                return Ok(());
            }
            if resp_parts[0] == consts::reply::UNREGISTERED_HOST {
                local_write_half
                    .write_line(consts::reply::UNREGISTERED_REMOTE)
                    .await?;
                return Ok(());
            }
            if resp_parts[0] == consts::reply::NO_PORT_AVAILABLE {
                local_write_half.write_line(consts::reply::REMOTE_NO_PORT_AVAILABLE).await?;
                return Ok(());
            }
            if resp_parts[0] == consts::reply::PORT_CONFIRM && resp_parts.len() == 2 {
                if let Ok(port) = u16::from_str_radix(resp_parts[1], 10) {
                    return send_files(
                        local_write_half,
                        SocketAddr::from((remote_addr.ip(), port)),
                        files_paths,
                    )
                    .await;
                }
            }
            reply_unexpected(&mut local_write_half, remote_write_half).await?;
            return Ok(());
        }
    } else {
        local_write_half
            .write_line(format_smolstr!(
                "{} {}",
                consts::reply::UNREACHABLE_ADDRESS,
                remote_addr
            ))
            .await?;
    }
    Ok(())
}

async fn try_register_to_remote(remote_host: SocketAddr) -> CommonResult<()> {
    let remote_stream = TcpStream::connect(remote_host).await?;
    let (read_half, mut write_half) = remote_stream.into_split();
    if write_half.writable().await.is_ok() {
        write_half.write_line(consts::request::REG_ME).await?;
        if read_half.readable().await.is_ok() {
            let mut reader = BufReader::new(read_half).take(50);
            let mut line = String::new();
            if reader.read_line(&mut line).await? != 0 {
                let resp = line.trim();
                if resp == consts::reply::REGISTRATION_SUCCEEDED {
                    return Ok(());
                }
                if resp == consts::reply::REMOTE_REGISTRATION_UNSUPPORTED {
                    return Err(CommonError::FailureResponse(
                        consts::reply::REMOTE_REGISTRATION_UNSUPPORTED,
                    ));
                }
            }
        }
    }
    Err(CommonError::FailureResponse(
        consts::reply::REMOTE_REGISTRATION_FAILED,
    ))
}

async fn try_register_to_local(
    hostname: &str,
    host_addr: SocketAddr,
) -> CommonResult<Option<SocketAddr>> {
    let conf_store_lock = global::config_store().await;
    let mut config_store = conf_store_lock.write().await;
    let replaced = config_store.register_host(hostname, host_addr);
    config_store.save_to_file()?;
    Ok(replaced)
}

async fn reply_unexpected(
    local_write_half: &mut SendHalf,
    mut write_half: OwnedWriteHalf,
) -> tokio::io::Result<()> {
    local_write_half
        .write_line(consts::reply::UNEXPECTED_REMOTE_RESPONSE)
        .await?;
    local_write_half.flush().await?;
    write_half
        .write_line(consts::reply::UNEXPECTED_RESPONSE)
        .await?;
    write_half.flush().await?;
    write_half.forget();
    Ok(())
}

pub(crate) async fn handle_remote(stream: TcpStream, peer_addr: SocketAddr) -> CommonResult<()> {
    let (remote_read_half, mut remote_writer) = stream.into_split();
    let config_lock = global::config_store().await;
    if remote_read_half.readable().await.is_ok() {
        let mut remote_reader = BufReader::new(remote_read_half).take(10);
        let mut line = String::new();
        if remote_reader.read_line(&mut line).await? != 0 && !line.trim().is_empty() {
            let line_parts: Vec<&str> = line.trim().split(' ').collect();
            match line_parts[0] {
                consts::request::PORT_EXPECTED if line_parts.len() == 2 => {
                    if config_lock.read().await.check_addr_registered(peer_addr) {
                        let client_res = try_create_client_ipc_stream().await;
                        let mut recv_dir = global::config_store()
                            .await
                            .read()
                            .await
                            .receive_dir()
                            .to_owned();
                        if let Ok(client_stream) = client_res {
                            let (client_read_half, mut client_writer) = client_stream.split();
                            client_writer
                                .write_line(consts::request::FILES_RECV)
                                .await?;
                            let mut client_reader =
                                BufReader::new(client_read_half).take(consts::FILE_PATH_LIMIT + 8);
                            let mut client_resp = String::new();
                            if client_reader.read_line(&mut client_resp).await? != 0
                                && !line.trim().is_empty()
                            {
                                let parts: Vec<&str> = line.trim().split(' ').collect();
                                if parts[0] == consts::reply::REJECT {
                                    remote_writer
                                        .write_line(consts::reply::RECV_REFUSED)
                                        .await?;
                                    return Ok(());
                                }
                                if parts[0] == consts::reply::ACCEPT && parts.len() == 2 {
                                    let p = PathBuf::from(parts[1]);
                                    if !p.is_dir() {
                                        client_writer
                                            .write_line(consts::reply::INVALID_RECV_DIR)
                                            .await?;
                                        client_writer.shutdown().await?;
                                        drop(client_reader);
                                    } else {
                                        recv_dir = p;
                                    }
                                }
                            } else {
                                client_writer
                                    .write_line(consts::reply::UNEXPECTED_RESPONSE)
                                    .await?;
                                client_writer.shutdown().await?;
                                drop(client_reader);
                            }
                        }
                        if let Ok(expected_port) = u16::from_str_radix(line_parts[1], 10) {
                            if let Ok(l) = try_create_listener(expected_port).await {
                                let actual_port = l.local_addr()?.port();
                                tokio::spawn(receive_files(l, peer_addr.ip(), recv_dir));
                                remote_writer
                                    .write_line(format_smolstr!(
                                        "{} {}",
                                        consts::reply::PORT_CONFIRM,
                                        actual_port
                                    ))
                                    .await?;
                            } else {
                                remote_writer
                                    .write_line(consts::reply::NO_PORT_AVAILABLE)
                                    .await?;
                            }
                        } else {
                            remote_writer
                                .write_line(consts::reply::INVALID_PORT)
                                .await?;
                        }
                    } else {
                        remote_writer
                            .write_line(consts::reply::UNREGISTERED_HOST)
                            .await?;
                    }

                    return Ok(());
                }
                consts::request::REG_ME => {
                    if let Ok(stream) = try_create_client_ipc_stream().await {
                        let (client_read_half, mut client_write_half) = stream.split();
                        if client_write_half
                            .write_line(format_smolstr!(
                                "{} {}",
                                consts::request::REG_FROM,
                                peer_addr
                            ))
                            .await
                            .is_ok()
                        {
                            let mut reader =
                                BufReader::with_capacity(50, client_read_half).take(50);
                            line.clear();
                            if reader.read_line(&mut line).await? != 0 && !line.trim().is_empty() {
                                let resp = line.trim();
                                if resp == consts::reply::REGISTRATION_SUCCEEDED {
                                    remote_writer
                                        .write_line(consts::reply::REGISTRATION_SUCCEEDED)
                                        .await?;
                                    return Ok(());
                                }
                                if resp == consts::reply::REGISTRATION_REFUSED {
                                    remote_writer
                                        .write_line(consts::reply::REMOTE_REGISTRATION_REFUSED)
                                        .await?;
                                    return Ok(());
                                }
                                if resp == consts::reply::CLIENT_REGISTRATION_FAILED {
                                    remote_writer
                                        .write_line(consts::reply::REMOTE_REGISTRATION_FAILED)
                                        .await?;
                                    return Ok(());
                                }
                            }
                        }
                    }
                    remote_writer
                        .write_line(consts::reply::REMOTE_REGISTRATION_UNSUPPORTED)
                        .await?;
                    return Ok(());
                }
                _ => (),
            }
        }
    }
    remote_writer
        .write_line(consts::reply::INVALID_REQUEST)
        .await?;
    Ok(())
}

async fn try_create_listener(port: u16) -> CommonResult<tokio::net::TcpListener> {
    for p in port..u16::MAX {
        if let Ok(l) = tokio::net::TcpListener::bind((Ipv4Addr::UNSPECIFIED, p)).await {
            return Ok(l);
        }
    }
    for p in port..1 {
        if let Ok(l) = tokio::net::TcpListener::bind((Ipv4Addr::UNSPECIFIED, p)).await {
            return Ok(l);
        }
    }
    Err(CommonError::FailureResponse(
        consts::reply::NO_PORT_AVAILABLE,
    ))
}

async fn send_files(
    mut local_write_half: SendHalf,
    dest_addr: SocketAddr,
    files_paths: Vec<PathBuf>,
) -> tokio::io::Result<()> {
    let dest_stream = TcpStream::connect(dest_addr).await?;
    let (read_half, write_half) = dest_stream.into_split();
    let mut dest_writer = BufWriter::new(write_half);
    let start_flag_with_line =
        constcat::concat!(consts::trans_flag::TRANSFER_START, consts::LINE_SEP);
    dest_writer.write_line(start_flag_with_line).await?;
    local_write_half.write_line(start_flag_with_line).await?;
    for p in &files_paths {
        let name_cow = p.file_name().unwrap().to_string_lossy();
        let name = if name_cow.len() >= consts::FILE_NAME_LENGTH_LIMIT {
            name_cow[0..consts::FILE_NAME_LENGTH_LIMIT].to_smolstr()
        } else {
            name_cow.to_smolstr()
        };

        let mut f = File::open(&p).await?;
        let file_size = f.metadata().await.unwrap().len();
        dest_writer
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
            dest_writer.write_all(&buf[0..read_size]).await?;
            dest_writer.flush().await?;
            local_write_half
                .write_line(format_smolstr!(
                    "{} {}:{}",
                    consts::reply::PROGRESS,
                    name_cow,
                    size_count as f64 / file_size as f64
                ))
                .await?;
        }
        dest_writer.write_all(consts::LINE_SEP.as_bytes()).await?;
    }
    local_write_half
        .write_line(consts::trans_flag::TRANSFER_END)
        .await?;
    dest_writer
        .write_line(consts::trans_flag::TRANSFER_END)
        .await?;
    let mut reader = BufReader::new(read_half).take(10);
    let mut line = String::new();
    if reader.read_line(&mut line).await? != 0 && !line.trim().is_empty() {
        let recv_resp: Vec<&str> = line.split(' ').collect();
        if recv_resp.len() == 2 && recv_resp[0] == consts::reply::RECEIVED {
            if let Ok(recv_count) = u8::from_str_radix(recv_resp[1], 10) {
                if recv_count as usize == files_paths.len() {
                    local_write_half
                        .write_line(consts::reply::ALL_FILES_SUCCEEDED)
                        .await?;
                    return Ok(());
                } else {
                    local_write_half
                        .write_line(format_smolstr!(
                            "{} {}",
                            consts::reply::FILES_SUCCEEDED,
                            recv_count
                        ))
                        .await?;
                    return Ok(());
                }
            }
        }
    }
    local_write_half
        .write_line(consts::reply::UNEXPECTED_SEND_RESPONSE)
        .await?;
    dest_writer
        .write_line(consts::reply::UNEXPECTED_SEND_RESPONSE)
        .await?;
    Ok(())
}

async fn receive_files(
    listener: TcpListener,
    send_host_ip: IpAddr,
    mut recv_dir: PathBuf,
) -> tokio::io::Result<()> {
    loop {
        let (stream, peer_addr) = listener.accept().await?;
        if peer_addr.ip() == send_host_ip {
            let (read_half, mut write_half) = stream.into_split();
            let mut reader =
                BufReader::new(read_half).take(consts::trans_flag::TRANSFER_START.len() as u64 + 2);
            let mut line = String::new();
            if reader.read_line(&mut line).await? != 0
                && line.trim() == consts::trans_flag::TRANSFER_START
            {
                let mut files_count: u32 = 0;

                line.clear();
                reader.set_limit(4);
                while reader.read_line(&mut line).await? != 0 && line.trim().is_empty() {
                    reader.set_limit(consts::FILE_NAME_LENGTH_LIMIT as u64 + 30);
                    line.clear();
                    if reader.read_line(&mut line).await? != 0 {
                        let trimmed_line = line.trim();
                        if trimmed_line == consts::trans_flag::TRANSFER_END {
                            if files_count > 0 {
                                write_half
                                    .write_line(format_smolstr!(
                                        "{} {}",
                                        consts::reply::RECEIVED,
                                        files_count
                                    ))
                                    .await?;
                                return Ok(());
                            } else {
                                break;
                            }
                        }
                        let parts: Vec<&str> = trimmed_line.split(':').collect();
                        if parts.len() == 2 {
                            let name = parts[0];
                            if let Ok(file_size) = usize::from_str_radix(parts[1], 10) {
                                reader.set_limit(file_size as u64);
                                recv_dir.push(name);
                                let f = RwLock::new(File::create(&recv_dir).await?);
                                let mut file_writer = f.write().await;
                                loop {
                                    let mut buf = [0; consts::FILE_TRANS_BUF_SIZE];
                                    let read_size = reader.read(&mut buf).await?;
                                    if read_size == 0 {
                                        break;
                                    }
                                    file_writer.write_all(&buf[0..read_size]).await?;
                                }
                                file_writer.flush().await?;
                                files_count += 1;
                                line.clear();
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                if files_count > 0 {
                    write_half
                        .write_line(format_smolstr!(
                            "{} {}:{}",
                            consts::reply::UNEXPECTED_END_FLAG,
                            consts::reply::RECEIVED,
                            files_count
                        ))
                        .await?;
                    return Ok(());
                }
            }
            write_half
                .write_line(consts::reply::INVALID_REQUEST)
                .await?;
            return Ok(());
        }
        tokio::task::yield_now().await;
    }
}

async fn try_create_client_ipc_stream() -> std::io::Result<interprocess::local_socket::tokio::Stream>
{
    interprocess::local_socket::tokio::Stream::connect(
        global::client_ipc_socket_name()
            .to_ns_name::<GenericNamespaced>()
            .unwrap(),
    )
    .await
}

#[cfg(test)]
mod tests {

    #[test]
    fn wrapping_add_test() {
        let max_u16 = u16::MAX;
        assert_eq!(1, max_u16.wrapping_add(2));
    }
}
