use std::{ffi::OsStr, net::SocketAddr, path::PathBuf};

use interprocess::local_socket::{
    traits::tokio::Listener, GenericNamespaced, ListenerOptions, NamespacedNameType, ToNsName,
};
use smol_str::{SmolStr, ToSmolStr};
use tokio::{net::TcpListener, task::JoinSet};

use crate::{config::Config, consts, global, handler};

fn join_set() -> &'static mut JoinSet<()> {
    static mut JOIN_SET: Option<JoinSet<()>> = None;
    unsafe { JOIN_SET.get_or_insert(JoinSet::new()) }
}

pub struct Server {
    server_ipc_sock_name: SmolStr,
    client_ipc_sock_name: SmolStr,
    log_to_file: bool,
    log_dir: PathBuf,
    log_level: log::LevelFilter,
    config: Config,
    use_config_file: bool,
}

impl Default for Server {
    fn default() -> Self {
        Self {
            server_ipc_sock_name: SmolStr::new_inline(consts::DEFAULT_SERVER_IPC_SOCKET_NAME),
            client_ipc_sock_name: SmolStr::new_inline(consts::DEFAULT_CLIENT_IPC_SOCKET_NAME),
            log_to_file: false,
            log_level: log::LevelFilter::Warn,
            log_dir: global::default_log_dir().to_owned(),
            config: Config::default(),
            use_config_file: false,
        }
    }
}

impl Server {
    fn init_logger(log_to_file: bool) -> anyhow::Result<()> {
        let mut log_builder = env_logger::builder();
        if log_to_file {
            log_builder.target(env_logger::Target::Pipe(Box::new(global::open_log_file())));
        } else {
            log_builder.target(env_logger::Target::Stdout);
        }
        log_builder
            .format_level(true)
            .format_module_path(true)
            .init();
        Ok(())
    }

    pub fn server_ipc_socket_name(&mut self, server_ipc_sock_name: &str) -> &mut Self {
        self.server_ipc_sock_name = Self::checked_ipc_socket_name(server_ipc_sock_name);
        self
    }

    fn checked_ipc_socket_name(name: &str) -> SmolStr {
        if name.to_ns_name::<GenericNamespaced>().is_ok() {
            return name.to_smolstr();
        }
        consts::DEFAULT_CLIENT_IPC_SOCKET_NAME.to_smolstr()
    }

    pub fn client_ipc_socket_name(&mut self, client_ipc_sock_name: &str) -> &mut Self {
        self.client_ipc_sock_name = Self::checked_ipc_socket_name(client_ipc_sock_name);
        self
    }

    pub fn listener_port(&mut self, port: u16) -> &mut Self {
        self.config.set_listener_port(port);
        self
    }

    pub fn max_log_level(&mut self, level: log::LevelFilter) -> &mut Self {
        self.log_level = level;
        self
    }

    pub fn log_dir<P: Into<PathBuf>>(&mut self, log_dir: P) -> &mut Self {
        self.log_dir = Self::checked_log_dir(log_dir.into());
        self
    }

    fn checked_log_dir(log_dir: PathBuf) -> PathBuf {
        if !log_dir.is_dir() {
            return global::default_log_dir().to_owned();
        }
        log_dir
    }

    pub fn use_config_file<P: Into<std::path::PathBuf>>(
        &mut self,
        config_file_path: P,
    ) -> anyhow::Result<&mut Self> {
        global::set_config_path(config_file_path.into())?;
        self.use_config_file = true;
        Ok(self)
    }

    pub fn num_workers(&mut self, n: u8) -> &mut Self {
        self.config.set_num_workers(n);
        self
    }

    pub fn log_to_file(&mut self) -> &mut Self {
        self.log_to_file = true;
        self
    }

    pub fn preset_receive_dir<P: Into<PathBuf>>(&mut self, receive_dir: P) -> &mut Self {
        self.config.set_receive_dir(receive_dir);
        self
    }

    pub fn add_reg_host(&mut self, hostname: &str, host: SocketAddr) -> &mut Self {
        self.config.register_host(hostname, host);
        self
    }

    pub fn start(self) -> anyhow::Result<()> {
        tokio::runtime::Builder::new_multi_thread()
            .enable_io()
            .build()
            .unwrap()
            .block_on(self.start_inner())
    }

    async fn try_join() {
        while join_set().len() > global::config_store().await.read().await.num_workers() as usize {
            if let Some(Err(e)) = join_set().join_next().await {
                log::error!("A local request handler task join failed: {}", e);
            }
        }
    }

    fn try_create_default_ipc_server(
    ) -> std::io::Result<interprocess::local_socket::tokio::Listener> {
        let ipc_name = consts::DEFAULT_SERVER_IPC_SOCKET_NAME
            .to_ns_name::<GenericNamespaced>()
            .unwrap();
        global::set_server_ipc_socket_name(consts::DEFAULT_SERVER_IPC_SOCKET_NAME.to_smolstr());
        ListenerOptions::new().name(ipc_name).create_tokio()
    }

    async fn start_local_listener() {
        let name_str = global::server_ipc_socket_name();
        let ipc_sock_name = GenericNamespaced::map(OsStr::new(name_str).into()).unwrap();
        let mut listener_res = ListenerOptions::new().name(ipc_sock_name).create_tokio();
        if listener_res.is_err() && name_str != consts::DEFAULT_SERVER_IPC_SOCKET_NAME {
            log::warn!("Create local listener failed by using specified IPC socket name: {name_str}. Try to create it using default name...");
            listener_res = Self::try_create_default_ipc_server();
        }
        if let Ok(local_listener) = listener_res {
            loop {
                let conn = match local_listener.accept().await {
                    Ok(c) => c,
                    Err(e) => {
                        log::warn!("There was an error with an incoming connection: {}", e);
                        continue;
                    }
                };
                Self::try_join().await;
                join_set().spawn(async move {
                    if let Err(e) = handler::handle_local(conn).await {
                        log::error!(
                            "Error occurred while handling a local process connection: {}",
                            e
                        );
                    }
                });
            }
        } else {
            let err = listener_res.unwrap_err();
            if err.kind() == tokio::io::ErrorKind::AddrInUse {
                log::error!("Error: could not start server because the socket file is occupied. Please check if {} is in use by another process and try again.", consts::DEFAULT_SERVER_IPC_SOCKET_NAME);
            } else {
                log::error!("Error occurred while create ipc listener: {}", err);
            }
            std::process::exit(1);
        }
    }

    async fn start_inner(mut self) -> anyhow::Result<()> {
        global::set_log_dir(self.log_dir);
        Self::init_logger(self.log_to_file)?;
        if self.use_config_file {
            self.config = global::config_store().await.read().await.clone_inner();
        }
        let remote_listener;
        let listen_res = TcpListener::bind(self.config.listener_addr).await;
        if let Err(e) = listen_res {
            if self.config.listener_addr == consts::DEFAULT_LISTENER_ADDR {
                return Err(e.into());
            }
            remote_listener = TcpListener::bind(consts::DEFAULT_LISTENER_ADDR).await?;
        } else {
            remote_listener = listen_res.unwrap();
        }
        let local_addr = remote_listener.local_addr().unwrap();
        self.config.set_listener_addr(local_addr);
        let conf_store_lock = global::config_store().await;
        let mut config_store = conf_store_lock.write().await;
        config_store.set_config(self.config);
        config_store.save_to_file()?;
        ctrlc::set_handler(|| {
            println!("CtrlC Pressed, Exiting forced now!");
            std::process::exit(0);
        })
        .expect("Set Ctrl+C event handler failed!");
        global::set_server_ipc_socket_name(self.server_ipc_sock_name);
        global::set_client_ipc_socket_name(self.client_ipc_sock_name);
        tokio::spawn(Self::start_local_listener());
        loop {
            match remote_listener.accept().await {
                Ok((socket, addr)) => {
                    Self::try_join().await;
                    join_set().spawn(async move {
                        if let Err(e) = handler::handle_remote(socket, addr).await {
                            log::error!("Error occurred while handling a remote connection: {}", e);
                        }
                    });
                }
                Err(e) => {
                    log::error!("Accept connection error: {}", e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {}
