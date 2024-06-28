use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
};

use interprocess::local_socket::{
    tokio::{SendHalf, Stream as LocalStream},
    traits::tokio::Stream,
};
use smol_str::ToSmolStr;
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
    net::{tcp::OwnedWriteHalf, TcpListener, TcpStream},
    sync::RwLock,
};

pub trait WriteLine {
    async fn write_line<S: AsRef<str>>(&mut self, str: S) -> std::io::Result<()>;
}

impl<W> WriteLine for W
where
    W: AsyncWriteExt + ?Sized + std::marker::Unpin,
{
    async fn write_line<S: AsRef<str>>(&mut self, str: S) -> std::io::Result<()> {
        self.write_all(str.as_ref().as_bytes()).await?;
        self.write_all(consts::LINE_SEP.as_bytes()).await?;
        self.flush().await
    }
}

use crate::{config::Config, consts, global, request_tag, response_tag};

pub(crate) async fn handle_local(stream: LocalStream) -> std::io::Result<()> {
    let (local_read_half, mut local_write_half) = stream.split();

    let mut local_reader =
        tokio::io::BufReader::new(local_read_half).take(consts::HOST_NAME_LENGTH_LIMIT as u64 + 50);
    let mut line = String::new();
    if local_reader.read_line(&mut line).await? != 0 {
        if let Some((command, arg)) = line.trim().split_once(consts::PIAR_SEP) {
            match command {
                request_tag::local::SHARE => {
                    if let Some(host) = global::config_store()
                        .await
                        .read()
                        .await
                        .get_addr_by_name(arg)
                    {
                        let mut recv_paths = Vec::with_capacity(consts::PATHS_NUM_PER_REQUEST);
                        local_reader.set_limit(consts::FILE_PATH_LIMIT);
                        line.clear();
                        while recv_paths.len() < consts::PATHS_NUM_PER_REQUEST
                            && local_reader.read_line(&mut line).await? != 0
                            && !line.trim().is_empty()
                        {
                            let path = PathBuf::from(line.trim());
                            if !path.is_file() {
                                local_write_half
                                    .write_line(response_tag::local::ANY_PATH_INVALID)
                                    .await?;
                                return Ok(());
                            }
                            recv_paths.push(path);
                            local_reader.set_limit(consts::FILE_PATH_LIMIT);
                            line.clear();
                        }
                        if !recv_paths.is_empty() {
                            handle_file_send(*host, local_write_half, recv_paths).await?;
                            return Ok(());
                        }
                    } else {
                        local_write_half
                            .write_line(response_tag::local::UNREGISTERED_HOSTNAME)
                            .await?;
                        return Ok(());
                    }
                }
                request_tag::local::REG => {
                    if let Some((hostname, addr_str)) = arg.trim().split_once(consts::PAIR_SEP) {
                        if !Config::check_hostname_valid(hostname) {
                            local_write_half
                                .write_line(response_tag::common::INVALID_HOSTNAME)
                                .await?;
                            return Ok(());
                        }
                        if let Ok(addr) = <SocketAddr as std::str::FromStr>::from_str(addr_str) {
                            if let Ok(option_ip) = try_register_to_local(hostname, addr).await {
                                if let Some(replaced) = option_ip {
                                    local_write_half
                                        .write_line(smol_str::format_smolstr!(
                                            "{} {}",
                                            response_tag::local::REPLACED_ADDR,
                                            replaced
                                        ))
                                        .await?;
                                } else {
                                    local_write_half
                                        .write_line(response_tag::common::REG_SUCCEEDED)
                                        .await?;
                                }
                            } else {
                                local_write_half
                                    .write_line(response_tag::local::REG_LOCAL_FAILED)
                                    .await?;
                            }
                            return Ok(());
                        }
                    }
                }
                _ => (),
            }
        }
    }
    local_write_half
        .write_line(response_tag::remote::INVALID_REQUEST)
        .await?;
    Ok(())
}

async fn try_register_to_local(
    hostname: &str,
    host_addr: SocketAddr,
) -> anyhow::Result<Option<SocketAddr>> {
    let conf_store_lock = global::config_store().await;
    let mut config_store = conf_store_lock.write().await;
    let replaced = config_store.register_host(hostname, host_addr);
    config_store.update_to_file()?;
    Ok(replaced)
}

fn checked_expected_port(port: u16) -> u16 {
    if port.wrapping_add(1) == 0 {
        port.wrapping_add(11)
    } else {
        port + 1
    }
}

async fn handle_file_send(
    remote_addr: SocketAddr,
    mut local_write_half: SendHalf,
    files_paths: Vec<PathBuf>,
) -> std::io::Result<()> {
    if let Ok(remote_stream) = TcpStream::connect(remote_addr).await {
        let (remote_read_half, mut remote_write_half) = remote_stream.into_split();
        remote_write_half
            .write_line(smol_str::format_smolstr!(
                "{} {}",
                request_tag::remote::PORT,
                checked_expected_port(remote_addr.port())
            ))
            .await?;
        let mut line = String::with_capacity(50);
        let mut remote_reader = BufReader::with_capacity(128, remote_read_half).take(50);
        if remote_reader.read_line(&mut line).await? != 0 {
            let mut resp_parts = line.trim().split(consts::PIAR_SEP);
            match resp_parts.next().unwrap() {
                response_tag::remote::UNREGISTERED_HOST => {
                    local_write_half
                        .write_line(response_tag::local::UNREGISTERED_REMOTE)
                        .await?
                }
                response_tag::remote::NO_AVAILABLE_PORT => {
                    local_write_half
                        .write_line(response_tag::local::NO_AVAILABLE_PORT_REMOTE)
                        .await?
                }
                response_tag::remote::PORT_CONFIRM if resp_parts.clone().count() == 2 => {
                    if let Ok(port) = resp_parts.next().unwrap().parse::<u16>() {
                        send_files(
                            local_write_half,
                            SocketAddr::from((remote_addr.ip(), port)),
                            files_paths,
                        )
                        .await?;
                    } else {
                        reply_unexpected_resp(&mut local_write_half, remote_write_half).await?;
                    }
                }
                _ => reply_unexpected_resp(&mut local_write_half, remote_write_half).await?,
            }
            return Ok(());
        }
        reply_unexpected_resp(&mut local_write_half, remote_write_half).await?;
    } else {
        local_write_half
            .write_line(smol_str::format_smolstr!(
                "{} {}",
                response_tag::local::UNREACHABLE_ADDRESS,
                remote_addr
            ))
            .await?;
    }

    Ok(())
}

async fn reply_unexpected_resp(
    local_write_half: &mut SendHalf,
    mut write_half: OwnedWriteHalf,
) -> tokio::io::Result<()> {
    local_write_half
        .write_line(response_tag::local::UNEXPECTED_REMOTE_RESP_TAG)
        .await?;
    local_write_half.flush().await?;
    write_half
        .write_line(response_tag::common::UNEXPECTED_RESP)
        .await?;
    write_half.flush().await?;
    write_half.forget();
    Ok(())
}

pub(crate) async fn handle_remote(stream: TcpStream, peer_addr: SocketAddr) -> anyhow::Result<()> {
    let (remote_read_half, mut remote_writer) = stream.into_split();
    let config_lock = global::config_store().await;
    if remote_read_half.readable().await.is_ok() {
        let first_line_length_limit = (request_tag::remote::PORT.len()
            + std::mem::size_of::<u16>()
            + consts::LINE_SEP.len()) as u64;
        let mut remote_reader = BufReader::new(remote_read_half).take(first_line_length_limit);
        let mut line = String::new();
        if remote_reader.read_line(&mut line).await? != 0 {
            if let Some((req_tag, arg)) = line.trim().split_once(consts::PIAR_SEP) {
                if req_tag == request_tag::remote::PORT {
                    if config_lock.read().await.check_addr_registered(peer_addr) {
                        if let Ok(expected_port) = arg.parse::<u16>() {
                            if let Some(l) = create_receive_listener(expected_port).await {
                                let actual_port = l.local_addr()?.port();
                                tokio::spawn(receive_files(l, peer_addr.ip()));
                                remote_writer
                                    .write_line(smol_str::format_smolstr!(
                                        "{} {}",
                                        response_tag::remote::PORT_CONFIRM,
                                        actual_port
                                    ))
                                    .await?;
                            } else {
                                remote_writer
                                    .write_line(response_tag::remote::NO_AVAILABLE_PORT)
                                    .await?;
                            }
                        } else {
                            remote_writer
                                .write_line(response_tag::remote::INVALID_PORT)
                                .await?;
                        }
                    } else {
                        remote_writer
                            .write_line(response_tag::remote::UNREGISTERED_HOST)
                            .await?;
                    }

                    return Ok(());
                }
            }
        }
    }
    remote_writer
        .write_line(response_tag::remote::INVALID_REQUEST)
        .await?;
    Ok(())
}

async fn send_files(
    mut local_write_half: SendHalf,
    dest_addr: SocketAddr,
    files_paths: Vec<PathBuf>,
) -> std::io::Result<()> {
    let dest_stream = TcpStream::connect(dest_addr).await?;
    let (remote_read_half, remote_write_half) = dest_stream.into_split();
    let mut dest_writer = BufWriter::new(remote_write_half);
    let start_flag_with_line =
        smol_str::format_smolstr!("{}{}", request_tag::send_flag::SEND_START, consts::LINE_SEP);
    dest_writer.write_line(&start_flag_with_line).await?;
    local_write_half.write_line(&start_flag_with_line).await?;
    for p in &files_paths {
        let name_cow = p.file_name().unwrap().to_string_lossy();
        let name = if name_cow.len() >= consts::FILE_NAME_LENGTH_LIMIT {
            unsafe {
                name_cow
                    .get_unchecked(0..consts::FILE_NAME_LENGTH_LIMIT)
                    .to_smolstr()
            }
        } else {
            name_cow.to_smolstr()
        };

        let mut f = File::open(&p).await?;
        let file_size = f.metadata().await.unwrap().len();
        dest_writer
            .write_line(smol_str::format_smolstr!("{}:{}", name, file_size))
            .await?;
        let mut size_count = 0;
        loop {
            let mut buf = [0_u8; consts::FILE_TRANS_BUF_SIZE];
            let read_size = f.read(&mut buf).await?;
            if read_size == 0 {
                break;
            }
            size_count += read_size;
            dest_writer
                .write_all(unsafe { buf.get_unchecked(0..read_size) })
                .await?;
            dest_writer.flush().await?;
            local_write_half
                .write_line(smol_str::format_smolstr!(
                    "{} {}:{}",
                    response_tag::local::PROGRESS,
                    name_cow,
                    size_count as f64 / file_size as f64
                ))
                .await?;
        }
        dest_writer.write_all(consts::LINE_SEP.as_bytes()).await?;
    }
    local_write_half
        .write_line(request_tag::send_flag::SEND_END)
        .await?;
    dest_writer
        .write_line(request_tag::send_flag::SEND_END)
        .await?;
    let mut reader = BufReader::new(remote_read_half).take(10);
    let mut line = String::new();
    if reader.read_line(&mut line).await? != 0 {
        let mut resp_parts = line.split(consts::PIAR_SEP);
        if resp_parts.next().unwrap() == response_tag::remote::FILES_RECEIVED
            && resp_parts.clone().count() == 2
        {
            if let Ok(recv_count) = resp_parts.next().unwrap().parse::<usize>() {
                if recv_count == files_paths.len() {
                    local_write_half
                        .write_line(response_tag::local::ALL_FILES_SENT_SUCCEEDED)
                        .await?;
                    return Ok(());
                } else {
                    local_write_half
                        .write_line(smol_str::format_smolstr!(
                            "{} {}",
                            response_tag::local::FILES_SENT_SUCCEEDED,
                            recv_count
                        ))
                        .await?;
                    return Ok(());
                }
            }
        }
    }
    local_write_half
        .write_line(response_tag::local::UNEXPECTED_SEND_RESPONSE)
        .await?;
    dest_writer
        .write_line(response_tag::local::UNEXPECTED_SEND_RESPONSE)
        .await?;
    Ok(())
}

async fn receive_files(listener: TcpListener, send_host_ip: IpAddr) -> std::io::Result<()> {
    loop {
        let (mut stream, peer_addr) = listener.accept().await?;
        if peer_addr.ip() == send_host_ip {
            let (read_half, mut write_half) = stream.into_split();
            let mut reader =
                BufReader::new(read_half).take(request_tag::send_flag::SEND_START.len() as u64 + 2);
            let mut line = String::new();
            if reader.read_line(&mut line).await? != 0
                && line.trim() == request_tag::send_flag::SEND_START
            {
                let mut files_count: u32 = 0;
                line.clear();
                reader.set_limit(4);
                let mut recv_dir = global::config_store()
                    .await
                    .read()
                    .await
                    .receive_dir()
                    .to_path_buf();
                while reader.read_line(&mut line).await? != 0 && line.trim().is_empty() {
                    reader.set_limit(consts::FILE_NAME_LENGTH_LIMIT as u64 + 40);
                    line.clear();
                    if reader.read_line(&mut line).await? == 0 {
                        break;
                    }
                    let trimmed_line = line.trim();
                    if trimmed_line == request_tag::send_flag::SEND_END {
                        if files_count == 0 {
                            break;
                        }
                        write_half
                            .write_line(smol_str::format_smolstr!(
                                "{} {}",
                                response_tag::remote::FILES_RECEIVED,
                                files_count
                            ))
                            .await?;
                        return Ok(());
                    }
                    let mut pair = trimmed_line.split(consts::PAIR_SEP);
                    if pair.clone().count() != 2 {
                        break;
                    }

                    let name = pair.next().unwrap();
                    let file_size_res = pair.next().unwrap().parse::<usize>();
                    if file_size_res.is_err() {
                        break;
                    }
                    reader.set_limit(file_size_res.unwrap() as u64);
                    recv_dir.push(name);
                    let f = RwLock::new(File::create(&recv_dir).await?);
                    let mut file_writer = f.write().await;
                    loop {
                        let mut buf = [0; consts::FILE_TRANS_BUF_SIZE];
                        let read_size = reader.read(&mut buf).await?;
                        if read_size == 0 {
                            break;
                        }
                        file_writer
                            .write_all(unsafe { buf.get_unchecked(0..read_size) })
                            .await?;
                    }
                    file_writer.flush().await?;
                    files_count += 1;
                    line.clear();
                }
                if files_count > 0 {
                    write_half
                        .write_line(smol_str::format_smolstr!(
                            "{} {}:{}",
                            response_tag::remote::UNEXPECTED_END_FLAG,
                            response_tag::remote::FILES_RECEIVED,
                            files_count
                        ))
                        .await?;
                    return Ok(());
                }
            }
            write_half
                .write_line(response_tag::remote::INVALID_REQUEST)
                .await?;
            return Ok(());
        }
        stream
            .write_line(response_tag::remote::INVALID_REQUEST)
            .await?;
        tokio::task::yield_now().await;
    }
}

async fn create_receive_listener(port: u16) -> Option<tokio::net::TcpListener> {
    for p in port..u16::MAX {
        if let Ok(l) = tokio::net::TcpListener::bind((Ipv4Addr::UNSPECIFIED, p)).await {
            return Some(l);
        }
    }
    for p in port..1 {
        if let Ok(l) = tokio::net::TcpListener::bind((Ipv4Addr::UNSPECIFIED, p)).await {
            return Some(l);
        }
    }
    None
}

// async fn try_connect_client(
//     client_ipc_name: SmolStr,
// ) -> std::io::Result<interprocess::local_socket::tokio::Stream> {
//     interprocess::local_socket::tokio::Stream::connect(
//         client_ipc_name.to_ns_name::<GenericNamespaced>().unwrap(),
//     )
//     .await
// }

#[cfg(test)]
mod tests {}
