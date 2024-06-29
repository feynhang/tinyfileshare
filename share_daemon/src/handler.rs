use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
};

use smol_str::ToSmolStr;
use tokio::{
    fs::File,
    io::{
        AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter,
    },
    net::{tcp::OwnedWriteHalf, TcpListener, TcpStream},
    sync::RwLock,
};

pub trait WriteLine {
    async fn write_line<S: AsRef<str>>(&mut self, str: S) -> std::io::Result<()>;
}

impl<W> WriteLine for W
where
    W: AsyncWriteExt + ?Sized + Unpin,
{
    async fn write_line<S: AsRef<str>>(&mut self, str: S) -> std::io::Result<()> {
        self.write_all(
            smol_str::format_smolstr!("{}{}", str.as_ref(), consts::LINE_SEP).as_bytes(),
        )
        .await?;
        self.flush().await
    }
}

use crate::{config::Config, consts, global, request_tag, response_tag};

pub(crate) async fn handle_local<S>(mut local_stream: S) -> std::io::Result<()>
where
    S: AsyncWrite + AsyncRead + Unpin,
    for<'a> &'a mut S: AsyncRead,
{
    // let mut local_write_half = stream.as_tokio_async_write();
    let mut local_reader =
        BufReader::new(&mut local_stream).take(consts::HOST_NAME_LENGTH_LIMIT as u64 + 50);
    let mut line = String::new();
    if local_reader.read_line(&mut line).await? != 0 {
        if let Some((command, arg)) = line.trim().split_once(consts::STARTLINE_SEP) {
            match command {
                request_tag::local::SHARE => {
                    if let Some(host) = global::config_store()
                        .await
                        .read()
                        .await
                        .get_addr_by_name(arg)
                    {
                        let mut recv_paths = Vec::with_capacity(consts::NUMBER_PATHS_PER_REQUEST);
                        local_reader.set_limit(consts::FILE_PATH_LIMIT);
                        line.clear();
                        while recv_paths.len() < consts::NUMBER_PATHS_PER_REQUEST
                            && local_reader.read_line(&mut line).await? != 0
                            && !line.trim().is_empty()
                        {
                            let path = PathBuf::from(line.trim());
                            if !path.is_file() {
                                local_stream
                                    .write_line(response_tag::local::ANY_PATH_INVALID)
                                    .await?;
                                return Ok(());
                            }
                            recv_paths.push(path);
                            local_reader.set_limit(consts::FILE_PATH_LIMIT);
                            line.clear();
                        }
                        if !recv_paths.is_empty() {
                            handle_file_send(*host, local_stream, recv_paths).await?;
                            return Ok(());
                        }
                    } else {
                        local_stream
                            .write_line(response_tag::local::UNREGISTERED_HOSTNAME)
                            .await?;
                        return Ok(());
                    }
                }
                request_tag::local::REG => {
                    if let Some((hostname, addr_str)) = arg.trim().split_once(consts::PAIR_SEP) {
                        if !Config::check_hostname_valid(hostname) {
                            local_stream
                                .write_line(response_tag::common::INVALID_HOSTNAME)
                                .await?;
                            return Ok(());
                        }
                        if let Ok(addr) = <SocketAddr as std::str::FromStr>::from_str(addr_str) {
                            if let Ok(option_ip) = try_register_to_local(hostname, addr).await {
                                if let Some(replaced) = option_ip {
                                    local_stream
                                        .write_line(smol_str::format_smolstr!(
                                            "{} {}",
                                            response_tag::local::REPLACED_ADDR,
                                            replaced
                                        ))
                                        .await?;
                                } else {
                                    local_stream
                                        .write_line(response_tag::common::REG_SUCCEEDED)
                                        .await?;
                                }
                            } else {
                                local_stream
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
    local_stream
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

async fn handle_file_send<S>(
    remote_addr: SocketAddr,
    mut local_write_half: S,
    files_paths: Vec<PathBuf>,
) -> std::io::Result<()>
where
    S: AsyncWrite + Unpin,
{
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
            let mut resp_parts = line.trim().split(consts::STARTLINE_SEP);
            match resp_parts.next().unwrap() {
                response_tag::remote::UNREGISTERED_HOST => {
                    local_write_half
                        .write_line(response_tag::local::REMOTE_UNREGISTERED)
                        .await?
                }
                response_tag::remote::NO_AVAILABLE_PORT => {
                    local_write_half
                        .write_line(response_tag::local::REMOTE_NO_AVAILABLE_PORT)
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

async fn reply_unexpected_resp<S>(
    local_write_half: &mut S,
    mut write_half: OwnedWriteHalf,
) -> tokio::io::Result<()>
where
    S: AsyncWrite + Unpin,
{
    local_write_half
        .write_line(response_tag::local::UNEXPECTED_REMOTE_RESP_TAG)
        .await?;
    write_half
        .write_line(response_tag::common::UNEXPECTED_RESP)
        .await?;
    write_half.flush().await?;
    write_half.forget();
    Ok(())
}

pub(crate) async fn handle_remote<S>(
    mut remote_stream: S,
    peer_addr: SocketAddr,
) -> anyhow::Result<()>
where
    S: AsyncWrite + AsyncRead + Unpin,
    for<'a> &'a mut S: AsyncRead,
{
    let first_line_length_limit = (request_tag::remote::PORT.len()
        + std::mem::size_of::<u16>()
        + consts::LINE_SEP.len()) as u64;
    let mut remote_reader = BufReader::new(&mut remote_stream).take(first_line_length_limit);
    let mut line = String::new();
    if let Ok(size) = remote_reader.read_line(&mut line).await {
        if size != 0 {
            if let Some((req_tag, arg)) = line.trim().split_once(consts::STARTLINE_SEP) {
                if req_tag == request_tag::remote::PORT {
                    if global::config_store()
                        .await
                        .read()
                        .await
                        .check_addr_registered(peer_addr)
                    {
                        if let Ok(expected_port) = arg.parse::<u16>() {
                            if let Some(l) = create_receive_listener(expected_port).await {
                                let actual_port = l.local_addr()?.port();
                                tokio::spawn(async move {
                                    if let Err(e) = receive_files(l, peer_addr.ip()).await {
                                        log::error!(
                                            "Error occurred in `receive_files`, error detail: {}",
                                            e
                                        );
                                    }
                                });
                                remote_stream
                                    .write_line(smol_str::format_smolstr!(
                                        "{} {}",
                                        response_tag::remote::PORT_CONFIRM,
                                        actual_port
                                    ))
                                    .await?;
                            } else {
                                remote_stream
                                    .write_line(response_tag::remote::NO_AVAILABLE_PORT)
                                    .await?;
                            }
                        } else {
                            remote_stream
                                .write_line(response_tag::remote::INVALID_PORT)
                                .await?;
                        }
                    } else {
                        remote_stream
                            .write_line(response_tag::remote::UNREGISTERED_HOST)
                            .await?;
                    }
                    return Ok(());
                }
            }
        }
    }
    if let Err(e) = remote_stream
        .write_line(response_tag::remote::INVALID_REQUEST)
        .await
    {
        log::error!("A remote connection maybe closed! Detail: {}", e);
    }
    Ok(())
}

async fn send_files<S>(
    mut local_write_half: S,
    dest_addr: SocketAddr,
    files_paths: Vec<PathBuf>,
) -> std::io::Result<()>
where
    S: AsyncWrite + Unpin,
{
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
        let mut resp_parts = line.split(consts::STARTLINE_SEP);
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
    for p in 3000..port {
        if let Ok(l) = tokio::net::TcpListener::bind((Ipv4Addr::UNSPECIFIED, p)).await {
            return Some(l);
        }
    }
    None
}

#[cfg(test)]
mod handler_tests {
    use std::{cell::RefCell, sync::Arc, task::Poll};

    use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, BufReader};

    use super::WriteLine;

    #[derive(Debug)]
    struct MockStream {
        data_s2c: Vec<u8>,
        data_c2s: Vec<u8>,
    }

    impl MockStream {
        pub fn new() -> Self {
            Self {
                data_c2s: vec![],
                data_s2c: vec![],
            }
        }

        pub fn into_split(self) -> (ClientStream, ServerStream) {
            println!("split inner");
            let arc = Arc::new(RefCell::new(self));
            (
                ClientStream {
                    inner_stream: arc.clone(),
                },
                ServerStream { inner_stream: arc },
            )
        }
    }

    #[derive(Debug, Clone)]
    struct ClientStream {
        inner_stream: Arc<RefCell<MockStream>>,
    }

    #[derive(Debug, Clone)]
    struct ServerStream {
        inner_stream: Arc<RefCell<MockStream>>,
    }

    impl Unpin for MockStream {}

    impl AsyncWrite for ClientStream {
        fn poll_write(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
            buf: &[u8],
        ) -> std::task::Poll<Result<usize, std::io::Error>> {
            let mut inner_stream = self.inner_stream.as_ref().borrow_mut();
            inner_stream.data_c2s = Vec::from(buf);
            Poll::Ready(Ok(buf.len()))
        }

        fn poll_flush(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Result<(), std::io::Error>> {
            Poll::Ready(Ok(()))
        }

        fn poll_shutdown(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Result<(), std::io::Error>> {
            Poll::Ready(Ok(()))
        }
    }

    impl AsyncRead for ClientStream {
        fn poll_read(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            let mut inner_stream = self.inner_stream.as_ref().borrow_mut();
            if inner_stream.data_s2c.len() > 0 {
                let size = usize::min(inner_stream.data_s2c.len(), buf.remaining());
                buf.put_slice(&inner_stream.data_s2c[..size]);
                inner_stream.data_s2c.drain(..size);
            }
            Poll::Ready(Ok(()))
        }
    }

    impl AsyncWrite for ServerStream {
        fn poll_write(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
            buf: &[u8],
        ) -> Poll<Result<usize, std::io::Error>> {
            let mut inner_stream = self.inner_stream.as_ref().borrow_mut();
            inner_stream.data_s2c = Vec::from(buf);
            Poll::Ready(Ok(buf.len()))
        }

        fn poll_flush(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> Poll<Result<(), std::io::Error>> {
            Poll::Ready(Ok(()))
        }

        fn poll_shutdown(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> Poll<Result<(), std::io::Error>> {
            Poll::Ready(Ok(()))
        }
    }

    impl AsyncRead for ServerStream {
        fn poll_read(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> Poll<std::io::Result<()>> {
            let mut inner_stream = self.inner_stream.as_ref().borrow_mut();
            if inner_stream.data_c2s.len() > 0 {
                let size = usize::min(inner_stream.data_c2s.len(), buf.remaining());
                buf.put_slice(&inner_stream.data_c2s[..size]);
                inner_stream.data_c2s.drain(..size);
            }
            Poll::Ready(Ok(()))
        }
    }

    const START_LINE: &str = "GET / HTTP/1.1";
    const HELLO_RESPONSE: &str = "<html><head></head><body><h1>Hello</h1></body></html>";

    async fn simple_handle_connection(mut server_stream: ServerStream) -> std::io::Result<()> {
        println!("into simple_handle_connection");
        let mut line = String::new();
        let mut reader = BufReader::new(&mut server_stream).take(20);
        let read_size = reader.read_line(&mut line).await?;
        println!("read a line finished, line = {}", line.trim());
        if read_size != 0 && line.trim() == START_LINE.trim() {
            println!("read a line and its equals START_LINE: \"{}\"", START_LINE);
            println!("write content =  {}", HELLO_RESPONSE);
            server_stream.write_line(HELLO_RESPONSE).await?;
            println!("write response finished!");
            println!(
                "inner stream data_s2c = {}",
                String::from_utf8_lossy(
                    server_stream
                        .inner_stream
                        .as_ref()
                        .borrow()
                        .data_s2c
                        .as_slice()
                )
            );
        }
        Ok(())
    }

    #[tokio::test(flavor = "current_thread")]
    async fn simple_handle_test() {
        println!("before initial");
        let stream = MockStream::new();
        println!("stream initial finished");
        let (mut c, s) = stream.into_split();
        println!("split stream succeeded...");
        let mut res = c.write_line(START_LINE).await;
        assert!(res.is_ok());
        println!(
            "client write startline finished, inner data = {}",
            String::from_utf8_lossy(c.inner_stream.as_ref().borrow().data_c2s.as_slice())
        );
        res = simple_handle_connection(s).await;
        assert!(res.is_ok());
        let mut client_reader = BufReader::new(c).take(128);
        let mut resp = String::new();
        let read_res = client_reader.read_to_string(&mut resp).await;
        println!("read result = {}", &resp);
        assert!(read_res.is_ok());
        assert_eq!(resp.trim(), HELLO_RESPONSE);
    }
}
