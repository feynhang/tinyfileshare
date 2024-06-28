use std::{
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
};

use interprocess::local_socket::{
    traits::tokio::Listener, GenericNamespaced, ListenerOptions, ToNsName,
};
use smol_str::SmolStr;
use tokio::{net::TcpListener, task::JoinSet};

use crate::{
    config::Config,
    consts, global, handler,
};

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
    log_target: env_logger::Target,
    max_log_level: log::LevelFilter,
    config: Config,
    config_path: PathBuf,
}

impl Default for Server {
    fn default() -> Self {
        Self {
            max_log_level: log::LevelFilter::Info,
            log_target: env_logger::Target::Stdout,
            config: Config::default(),
            config_path: Config::default_config_path(),
        }
    }
}

impl Server {


    pub fn set_max_log_level(&mut self, level: log::LevelFilter) {
        self.max_log_level = level
    }

    pub fn max_log_level(&mut self, level: log::LevelFilter) -> &mut Self {
        self.max_log_level = level;
        self
    }

    pub fn log_target(&mut self, target: env_logger::Target) -> &mut Self {
        self.log_target = target;
        self
    }


    pub fn listener_port(&mut self, port: u16) -> &mut Self {
        self.config.set_listener_port(port);
        self
    }

    pub fn set_log_target(&mut self, target: env_logger::Target) {
        self.log_target = target;
    }

    pub fn set_ipc_socket_name(&mut self, ipc_socket_name: SmolStr) {
        self.config.set_ipc_socket_name(ipc_socket_name);
    }

    pub fn set_listener_port(&mut self, port: u16) {
        self.config.set_listener_port(port);
    }

    pub fn set_listener_addr(&mut self, addr: SocketAddr) {
        self.config.set_listener_addr(addr);
    }

    pub fn set_listener_ip(&mut self, ip: IpAddr) {
        self.config.set_listener_ip(ip);
    }

    pub fn load_config_file<P: AsRef<Path>>(&mut self, config_file_path: P) -> anyhow::Result<()> {
        let p = config_file_path.as_ref();
        let (ok, c) = Config::from_file(p)?.0.checked();
        if !ok {
            c.write_to_file(p)?;
        }
        self.config = c;
        p.clone_into(&mut self.config_path);
        Ok(())
    }

    pub fn set_num_workers(&mut self, n: u8) {
        self.config.set_num_workers(n);
    }

    pub fn set_save_dir<P: Into<PathBuf>>(&mut self, save_dir: P) {
        self.config.set_save_dir(save_dir);
    }

    pub fn register_host(&mut self, hostname: &str, host: SocketAddr) -> anyhow::Result<()> {
        if Config::check_hostname_valid(hostname) {
            self.config.register_host(hostname, host);
        }
        Err(anyhow::anyhow!("Invalid hostname!"))
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

    async fn try_create_default_ipc_server(
    ) -> std::io::Result<interprocess::local_socket::tokio::Listener> {
        let ipc_name = consts::DEFAULT_IPC_SOCK_NAME.to_ns_name::<GenericNamespaced>()?;
        global::config_store()
            .await
            .write()
            .await
            .set_ipc_socket_name(consts::DEFAULT_IPC_SOCK_NAME.into());
        ListenerOptions::new().name(ipc_name).create_tokio()
    }

    async fn start_local_listener() {
        let config_reader = global::config_store().await.read().await;
        let ipc_sock_name_str = config_reader.ipc_socket_name();
        let mut listener_res = ListenerOptions::new()
            .name(ipc_sock_name_str.to_ns_name::<GenericNamespaced>().unwrap())
            .create_tokio();
        if listener_res.is_err() && ipc_sock_name_str != consts::DEFAULT_IPC_SOCK_NAME {
            log::warn!("Create local listener failed by using specified IPC socket name: `{}`. Try to create it using default name...", ipc_sock_name_str);
            listener_res = Self::try_create_default_ipc_server().await;
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
                log::error!("Error: could not start server because the socket file is occupied. Please check if {} is in use by another process and try again.", global::config_store().await.read().await.ipc_socket_name());
            } else {
                log::error!("Error occurred while create ipc listener: {}", err);
            }
            std::process::exit(1);
        }
    }


    async fn start_inner(self) -> anyhow::Result<()> {
        init_global_logger(self.log_target, self.max_log_level)?;
        let mut config = self.config;
        let preset_listener_addr = config.listener_addr();
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
        let local_addr = remote_listener.local_addr()?;
        log::info!("Server start at {}\n", local_addr);
        config.set_listener_addr(local_addr);
        let mut config_store = global::config_store().await.write().await;
        config_store.set_config(config);
        config_store.update_to_file()?;        
        if let Err(e) = ctrlc::set_handler(|| {
            println!("CtrlC Pressed, Exiting forced now!");
            std::process::exit(0);
        }) {
            log::warn!("Set CtrlC event failed! detail: {e}");
        }

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
