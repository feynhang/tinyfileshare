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

pub(crate) fn init_global_logger(
    log_target: env_logger::Target,
    max_log_level: log::LevelFilter,
) -> anyhow::Result<()> {
    let mut log_builder = env_logger::builder();
    log_builder
        .target(log_target)
        .filter_level(max_log_level)
        .format_level(true)
        .format_module_path(true)
        .init();
    Ok(())
}

pub struct Server {
    server_ipc_sock_name: SmolStr,
    client_ipc_sock_name: SmolStr,
    log_target: env_logger::Target,
    max_log_level: log::LevelFilter,
    config: Config,
    config_path: Option<PathBuf>,
}

impl Default for Server {
    fn default() -> Self {
        Self {
            server_ipc_sock_name: SmolStr::new_inline(consts::DEFAULT_SERVER_IPC_SOCK_NAME),
            client_ipc_sock_name: SmolStr::new_inline(consts::DEFAULT_CLIENT_IPC_SOCK_NAME),
            max_log_level: log::LevelFilter::Info,
            log_target: env_logger::Target::Stdout,
            config: Config::default(),
            config_path: None,
        }
    }
}

impl Server {

    pub fn max_log_level(&mut self, level: log::LevelFilter) -> &mut Self {
        self.max_log_level = level;
        self
    }

    pub fn log_target(&mut self, target: env_logger::Target) -> &mut Self {
        self.log_target = target;
        self
    }

    
    fn checked_ipc_socket_name(name: &str) -> SmolStr {
        if name.to_ns_name::<GenericNamespaced>().is_ok() {
            return name.to_smolstr();
        }
        consts::DEFAULT_CLIENT_IPC_SOCK_NAME.to_smolstr()
    }

    pub fn server_ipc_socket_name(&mut self, server_ipc_sock_name: &str) -> &mut Self {
        self.server_ipc_sock_name = Self::checked_ipc_socket_name(server_ipc_sock_name);
        self
    }

    pub fn client_ipc_socket_name(&mut self, client_ipc_sock_name: &str) -> &mut Self {
        self.client_ipc_sock_name = Self::checked_ipc_socket_name(client_ipc_sock_name);
        self
    }

    pub fn listener_port(&mut self, port: u16) -> &mut Self {
        self.config.set_listener_port(port);
        self
    }

   

    pub fn load_config_file(&mut self, config_file_path: PathBuf) -> anyhow::Result<&mut Self> {
        self.config_path = Some(config_file_path);
        Ok(self)
    }

    pub fn num_workers(&mut self, n: u8) -> &mut Self {
        self.config.set_num_workers(n);
        self
    }

    pub fn preset_receive_dir<P: Into<PathBuf>>(&mut self, receive_dir: P) -> &mut Self {
        self.config.set_receive_dir(receive_dir);
        self
    }

    pub fn add_host_to_local(
        &mut self,
        hostname: &str,
        host: SocketAddr,
    ) -> anyhow::Result<&mut Self> {
        if Config::check_hostname_valid(hostname) {
            self.config.register_host(hostname, host);
            return Ok(self);
        }
        Err(anyhow::anyhow!(
            "The length of hostname {} is out of max 16(bytes)!",
            hostname
        ))
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
        let ipc_name = consts::DEFAULT_SERVER_IPC_SOCK_NAME
            .to_ns_name::<GenericNamespaced>()
            .unwrap();
        global::set_server_ipc_sock_name(consts::DEFAULT_SERVER_IPC_SOCK_NAME.to_smolstr());
        ListenerOptions::new().name(ipc_name).create_tokio()
    }

    async fn start_local_listener() {
        let ipc_sock_name =
            GenericNamespaced::map(OsStr::new(global::server_ipc_sock_name()).into()).unwrap();
        let mut listener_res = ListenerOptions::new().name(ipc_sock_name).create_tokio();
        if listener_res.is_err()
            && global::server_ipc_sock_name() != consts::DEFAULT_SERVER_IPC_SOCK_NAME
        {
            log::warn!("Create local listener failed by using specified IPC socket name: {}. Try to create it using default name...", global::server_ipc_sock_name());
            listener_res = Self::try_create_default_ipc_server();
        }
        if let Ok(local_listener) = listener_res {
            log::info!("Local process listener start finished!");
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
                log::error!("Error: could not start server because the socket file is occupied. Please check if {} is in use by another process and try again.", global::server_ipc_sock_name());
            } else {
                log::error!("Error occurred while create ipc listener: {}", err);
            }
            std::process::exit(1);
        }
    }

    async fn start_inner(self) -> anyhow::Result<()> {
        init_global_logger(self.log_target, self.max_log_level)?;
        let conf_store_lock = global::config_store().await;
        let mut config_store = conf_store_lock.write().await;
        
        if let Some(config_path) = self.config_path {
            config_store.set_config_path(config_path);
            config_store.try_update_from_file()?;
        } else {
            config_store.set_config(self.config)?;
        }
        let preset_listener_addr = config_store.listener_addr();
        let remote_listener: TcpListener;
        let listen_res = TcpListener::bind(preset_listener_addr).await;
        if let Err(e) = listen_res {
            if preset_listener_addr == consts::DEFAULT_LISTENER_ADDR {
                return Err(e.into());
            }
            remote_listener = TcpListener::bind(consts::DEFAULT_LISTENER_ADDR).await?;
        } else {
            remote_listener = listen_res.unwrap();
        }
        let local_addr = remote_listener.local_addr().unwrap();
        log::info!("Server start at {}\n", local_addr);
        config_store.set_listener_addr(local_addr);
        config_store.update_to_file()?;
        
        if let Err(e) = ctrlc::set_handler(|| {
            println!("CtrlC Pressed, Exiting forced now!");
            std::process::exit(0);
        }) {
            log::warn!("Set CtrlC event failed! detail: {e}");
        }
        
        global::set_server_ipc_sock_name(self.server_ipc_sock_name);
        tokio::spawn(Self::start_local_listener());
        loop {
            match remote_listener.accept().await {
                Ok((stream, addr)) => {
                    Self::try_join().await;
                    join_set().spawn(async move {
                        if let Err(e) = handler::handle_remote(stream, addr).await {
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
